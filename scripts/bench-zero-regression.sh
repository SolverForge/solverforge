#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BASELINE=${BASELINE:-82ee720da76e}
BENCH_CPU=${BENCH_CPU:-10}
WARMUPS=${WARMUPS:-5}
TRIALS=${TRIALS:-51}
MEASURE_ITERATIONS=${MEASURE_ITERATIONS:-32}
CARTESIAN_MEASURE_ITERATIONS=${CARTESIAN_MEASURE_ITERATIONS:-256}
TOOLCHAIN=${TOOLCHAIN:-1.95.0}
RESULTS=${RESULTS:-"/tmp/solverforge-zero-regression-$(date +%Y%m%d-%H%M%S)"}
DEFAULT_CASES=(list_change list_swap nearby_change nearby_swap sublist_change sublist_swap filtering cartesian k_opt ruin construction_full construction_first_fit)
if [[ -n ${BENCH_CASES:-} ]]; then
    read -r -a CASES <<<"$BENCH_CASES"
else
    CASES=("${DEFAULT_CASES[@]}")
fi
BENCH_SOURCE="$ROOT/crates/solverforge-solver/benches/selector_cursor_gate.rs"

measure_iterations_for_case() {
    local case_name=$1
    if [[ $case_name == cartesian ]]; then
        printf '%s\n' "$CARTESIAN_MEASURE_ITERATIONS"
    else
        printf '%s\n' "$MEASURE_ITERATIONS"
    fi
}

if ! command -v taskset >/dev/null; then
    echo "taskset is required" >&2
    exit 2
fi
if [[ ! -r "/sys/devices/system/cpu/cpu${BENCH_CPU}/online" && ! -d "/sys/devices/system/cpu/cpu${BENCH_CPU}" ]]; then
    echo "CPU ${BENCH_CPU} is not present" >&2
    exit 2
fi
if [[ -r "/sys/devices/system/cpu/cpu${BENCH_CPU}/online" ]] && [[ $(<"/sys/devices/system/cpu/cpu${BENCH_CPU}/online") != 1 ]]; then
    echo "CPU ${BENCH_CPU} is offline" >&2
    exit 2
fi
if [[ -e "$RESULTS" ]]; then
    echo "RESULTS already exists; choose a fresh evidence directory: $RESULTS" >&2
    exit 2
fi

mkdir -p "$RESULTS/raw" "$RESULTS/harnesses" "$RESULTS/targets" "$RESULTS/binaries"
BASELINE_TREE="$RESULTS/baseline-tree"
BASELINE_REGISTERED=0
cleanup() {
    if [[ $BASELINE_REGISTERED == 1 ]]; then
        git -C "$ROOT" worktree remove --force "$BASELINE_TREE" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

BASELINE_COMMIT=$(git -C "$ROOT" rev-parse "${BASELINE}^{commit}")
git -C "$ROOT" worktree add --detach "$BASELINE_TREE" "$BASELINE_COMMIT" >/dev/null
BASELINE_REGISTERED=1

make_harness() {
    local name=$1
    local tree=$2
    local directory="$RESULTS/harnesses/$name"
    mkdir -p "$directory"
    cat >"$directory/Cargo.toml" <<EOF
[package]
name = "solverforge-selector-gate-$name"
version = "0.0.0"
edition = "2021"
publish = false

[features]
candidate = []

[[bin]]
name = "selector_cursor_gate"
path = "$BENCH_SOURCE"

[dependencies]
solverforge-core = { path = "$tree/crates/solverforge-core" }
solverforge-scoring = { path = "$tree/crates/solverforge-scoring" }
solverforge-solver = { path = "$tree/crates/solverforge-solver" }

[profile.release]
codegen-units = 1
incremental = false
EOF
    cargo "+$TOOLCHAIN" generate-lockfile --manifest-path "$directory/Cargo.toml" >/dev/null
}

make_harness baseline "$BASELINE_TREE"
make_harness candidate "$ROOT"

build_case_binary() {
    local side=$1
    local case_name=$2
    local manifest="$RESULTS/harnesses/$side/Cargo.toml"
    local target="$RESULTS/targets/$side"
    local feature_args=()
    if [[ $side == candidate ]]; then
        feature_args=(--features candidate)
    fi
    SOLVERFORGE_BENCH_CASE="$case_name" CARGO_TARGET_DIR="$target" \
        cargo "+$TOOLCHAIN" build --manifest-path "$manifest" --release --locked \
        "${feature_args[@]}" >/dev/null
    cp "$target/release/selector_cursor_gate" "$RESULTS/binaries/${side}-${case_name}"
}

for case_name in "${CASES[@]}"; do
    build_case_binary baseline "$case_name"
    build_case_binary candidate "$case_name"
done

GOVERNOR=unknown
if [[ -r "/sys/devices/system/cpu/cpu${BENCH_CPU}/cpufreq/scaling_governor" ]]; then
    GOVERNOR=$(<"/sys/devices/system/cpu/cpu${BENCH_CPU}/cpufreq/scaling_governor")
fi
SIBLINGS=unknown
if [[ -r "/sys/devices/system/cpu/cpu${BENCH_CPU}/topology/thread_siblings_list" ]]; then
    SIBLINGS=$(<"/sys/devices/system/cpu/cpu${BENCH_CPU}/topology/thread_siblings_list")
fi
CPU_MODEL=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)
CPU_MODEL=${CPU_MODEL:-unknown}
PERF_AVAILABLE=0
if command -v perf >/dev/null && perf stat -e instructions -- true >/dev/null 2>&1; then
    PERF_AVAILABLE=1
fi
CANDIDATE_DIRTY=false
if [[ -n $(git -C "$ROOT" status --porcelain) ]]; then
    CANDIDATE_DIRTY=true
fi
CANDIDATE_TREE_SHA256=$(
    cd "$ROOT"
    git ls-files -co --exclude-standard -z | sort -z |
        while IFS= read -r -d '' path; do
            if [[ -f $path ]]; then
                sha256sum -- "$path"
            else
                printf 'deleted  %s\n' "$path"
            fi
        done |
        sha256sum | cut -d' ' -f1
)

cat >"$RESULTS/environment.json" <<EOF
{"baseline_requested":"$BASELINE","baseline_commit":"$BASELINE_COMMIT","candidate_head":"$(git -C "$ROOT" rev-parse HEAD)","candidate_dirty":$CANDIDATE_DIRTY,"candidate_tree_sha256":"$CANDIDATE_TREE_SHA256","profile":"release(codegen-units=1,incremental=false)","case_isolated_binaries":true,"cpu_model":"$CPU_MODEL","affinity_cpu":$BENCH_CPU,"thread_siblings":"$SIBLINGS","governor":"$GOVERNOR","toolchain":"$TOOLCHAIN","warmups":$WARMUPS,"trials":$TRIALS,"measure_iterations":{"default":$MEASURE_ITERATIONS,"cartesian":$CARTESIAN_MEASURE_ITERATIONS},"perf_available":$PERF_AVAILABLE}
EOF

for case_name in "${CASES[@]}"; do
    BASELINE_BIN="$RESULTS/binaries/baseline-${case_name}"
    CANDIDATE_BIN="$RESULTS/binaries/candidate-${case_name}"
    case_iterations=$(measure_iterations_for_case "$case_name")
    for ((warmup = 0; warmup < WARMUPS; warmup++)); do
        taskset -c "$BENCH_CPU" "$BASELINE_BIN" "$case_name" "$case_iterations" >/dev/null
        taskset -c "$BENCH_CPU" "$CANDIDATE_BIN" "$case_name" "$case_iterations" >/dev/null
    done
done

run_trial() {
    local side=$1
    local binary=$2
    local case_name=$3
    local trial=$4
    local case_iterations
    case_iterations=$(measure_iterations_for_case "$case_name")
    local json_file="$RESULTS/raw/${side}-${case_name}.jsonl"
    local rss_file="$RESULTS/raw/${side}-${case_name}.rss"
    local perf_file="$RESULTS/raw/${side}-${case_name}-${trial}.perf.csv"
    local sample_file="$RESULTS/raw/sample-${side}-${case_name}-${trial}.json"
    local sample_rss="$RESULTS/raw/sample-${side}-${case_name}-${trial}.rss"

    /usr/bin/time -f '%M' -o "$sample_rss" \
        taskset -c "$BENCH_CPU" "$binary" "$case_name" "$case_iterations" >"$sample_file"
    if [[ $PERF_AVAILABLE == 1 ]]; then
        perf stat -x, -o "$perf_file" \
            -e instructions:u,cycles:u,branches:u,branch-misses:u,cache-references:u,cache-misses:u -- \
            taskset -c "$BENCH_CPU" "$binary" "$case_name" "$case_iterations" >/dev/null
    fi
    tr -d '\n' <"$sample_file" >>"$json_file"
    printf '\n' >>"$json_file"
    tr -d '[:space:]' <"$sample_rss" >>"$rss_file"
    printf '\n' >>"$rss_file"
}

for case_name in "${CASES[@]}"; do
    BASELINE_BIN="$RESULTS/binaries/baseline-${case_name}"
    CANDIDATE_BIN="$RESULTS/binaries/candidate-${case_name}"
    for ((trial = 0; trial < TRIALS; trial++)); do
        if ((trial % 2 == 0)); then
            run_trial baseline "$BASELINE_BIN" "$case_name" "$trial"
            run_trial candidate "$CANDIDATE_BIN" "$case_name" "$trial"
        else
            run_trial candidate "$CANDIDATE_BIN" "$case_name" "$trial"
            run_trial baseline "$BASELINE_BIN" "$case_name" "$trial"
        fi
    done
done

python3 - "$RESULTS" "$PERF_AVAILABLE" "${CASES[@]}" <<'PY'
import json
import math
import pathlib
import random
import statistics
import sys

root = pathlib.Path(sys.argv[1])
perf_available = sys.argv[2] == "1"
cases = sys.argv[3:]
metrics = ["wall_ns", "allocations", "allocated_bytes", "peak_live_bytes", "max_rss_kb"]
perf_metrics = [
    "instructions",
    "cycles",
    "branches",
    "branch-misses",
    "cache-references",
    "cache-misses",
]

def load_json(side, case):
    path = root / "raw" / f"{side}-{case}.jsonl"
    values = [json.loads(line) for line in path.read_text().splitlines() if line]
    rss = [int(line) for line in (root / "raw" / f"{side}-{case}.rss").read_text().splitlines()]
    if len(values) != len(rss):
        raise SystemExit(f"unaligned RSS samples for {side} {case}")
    for sample, max_rss in zip(values, rss):
        sample["max_rss_kb"] = max_rss
    return values

def load_perf(side, case, trial):
    path = root / "raw" / f"{side}-{case}-{trial}.perf.csv"
    result = {}
    if not path.exists():
        return result
    for line in path.read_text().splitlines():
        columns = line.split(",")
        if len(columns) < 3 or not columns[0].strip().replace(".", "", 1).isdigit():
            continue
        event = columns[2].strip().split(":", 1)[0]
        if event in perf_metrics:
            result[event] = float(columns[0].strip())
    return result

def bootstrap_upper(differences, seed):
    if not differences:
        return math.nan
    rng = random.Random(seed)
    medians = []
    for _ in range(10_000):
        medians.append(statistics.median(rng.choice(differences) for _ in differences))
    medians.sort()
    return medians[math.ceil(0.95 * len(medians)) - 1]

def interquartile_range(values):
    lower, _, upper = statistics.quantiles(values, n=4, method="inclusive")
    return upper - lower

failed = False
report = {"cases": {}, "passed": True}
for case_index, case in enumerate(cases):
    baseline = load_json("baseline", case)
    candidate = load_json("candidate", case)
    if len(baseline) != len(candidate):
        raise SystemExit(f"unaligned samples for {case}")
    baseline_identity = {(sample["candidate_count"], sample["order_hash"]) for sample in baseline}
    candidate_identity = {(sample["candidate_count"], sample["order_hash"]) for sample in candidate}
    if case == "construction_first_fit":
        baseline_order = {sample["order_hash"] for sample in baseline}
        candidate_order = {sample["order_hash"] for sample in candidate}
        baseline_work = {sample["candidate_count"] for sample in baseline}
        candidate_work = {sample["candidate_count"] for sample in candidate}
        semantic_match = (
            len(baseline_order) == 1
            and baseline_order == candidate_order
            and len(baseline_work) == 1
            and len(candidate_work) == 1
            and next(iter(candidate_work)) <= next(iter(baseline_work))
        )
    else:
        semantic_match = len(baseline_identity) == 1 and baseline_identity == candidate_identity
    case_report = {"semantic_match": semantic_match, "metrics": {}}
    if case == "construction_first_fit":
        case_report["generated_work"] = {
            "baseline": next(iter(baseline_work)),
            "candidate": next(iter(candidate_work)),
        }
        case_report["selected_trace_hash"] = sorted(candidate_order)[0]
    else:
        candidate_count, order_hash = sorted(candidate_identity)[0]
        case_report["candidate_identity"] = {
            "candidate_count": candidate_count,
            "order_hash": order_hash,
        }
    failed |= not semantic_match

    if perf_available:
        for trial in range(len(baseline)):
            baseline[trial].update(load_perf("baseline", case, trial))
            candidate[trial].update(load_perf("candidate", case, trial))
        metrics_for_case = metrics + [
            metric
            for metric in perf_metrics
            if all(metric in sample for sample in baseline + candidate)
        ]
    else:
        metrics_for_case = metrics

    for metric_index, metric in enumerate(metrics_for_case):
        baseline_values = [sample[metric] for sample in baseline]
        candidate_values = [sample[metric] for sample in candidate]
        differences = [after - before for before, after in zip(baseline_values, candidate_values)]
        baseline_median = statistics.median(baseline_values)
        candidate_median = statistics.median(candidate_values)
        upper = bootstrap_upper(differences, case_index * 100 + metric_index)
        passed = candidate_median <= baseline_median and upper <= 0
        failed |= not passed
        case_report["metrics"][metric] = {
            "baseline_median": baseline_median,
            "candidate_median": candidate_median,
            "baseline_iqr": interquartile_range(baseline_values),
            "candidate_iqr": interquartile_range(candidate_values),
            "delta_percent": (
                (candidate_median - baseline_median) * 100.0 / baseline_median
                if baseline_median
                else 0.0
            ),
            "paired_median_difference": statistics.median(differences),
            "paired_bootstrap_upper_95": upper,
            "passed": passed,
        }
    report["cases"][case] = case_report

report["passed"] = not failed
(root / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print(json.dumps(report, indent=2, sort_keys=True))
raise SystemExit(1 if failed else 0)
PY

echo "zero-regression evidence: $RESULTS"
