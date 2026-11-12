#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BASELINE=${BASELINE:-82ee720da76e}
BASELINE_CURSOR_API=${BASELINE_CURSOR_API:-0}
BENCH_CPU=${BENCH_CPU:-10}
WARMUPS=${WARMUPS:-5}
TRIALS=${TRIALS:-51}
# Documented hot-path budget (AGENTS.md: median throughput regression > 5%
# needs explicit approval). The gate rejects median wall/counter regressions
# beyond this budget; allocation counters must match exactly.
BENCH_TOLERANCE_PERCENT=${BENCH_TOLERANCE_PERCENT:-5.0}
# BENCH_OWN_SOURCES=1 compiles each side against its OWN tree's gate sources.
# Required when the two sides span a public-API generation change (the shared-
# source mode needs every API call to exist in both trees). Cross-tree hash
# comparison is then disabled: only exact counts and allocation metrics compare.
BENCH_OWN_SOURCES=${BENCH_OWN_SOURCES:-0}
# BENCH_ISOLATE=1 (default) gives BENCH_CPU and its hyperthread siblings to
# the measurement alone: an isolated cgroup cpuset partition moves every other
# task off them while trials run. Without it, other processes sharing the
# core's L2 move cache counters far more than the regression budget. Needs
# passwordless sudo; BENCH_ISOLATE=0 measures on a shared CPU and is recorded
# as such in environment.json.
BENCH_ISOLATE=${BENCH_ISOLATE:-1}
# Cache and branch behaviour is graded on Cachegrind's simulation, not on the
# hardware counters. The simulated L1 and last-level misses and branch
# mispredictions depend only on the binary and its input, so they repeat to
# within a count on this shared machine, while the same binary's hardware
# cache-references move by several percent between runs (an identical-source
# cartesian run measured +4.6%). Each side runs once per case under
# `setarch -R` (no address randomisation), from the equal-length paths below.
# The hardware counters stay in the report for diagnosis.
# The simulation repeats exactly for one binary, but it depends on where the
# linker places each function, and that placement follows symbol hashes: any
# change to a generic type name reorders functions whose bodies are untouched.
# Each side is therefore built in SIM_LAYOUTS layouts that differ only in
# `-C metadata`, and every simulated metric is taken as its median across
# them; layout 0 is the binary the timed trials run. SIM_JOBS simulations run
# at once, which does not change their counts.
# Across layouts of one source, simulated instructions, data-L1 misses and
# last-level misses hold within a fraction of a percent, so they are graded.
# Simulated instruction-L1 misses and branch mispredictions are reported only:
# the simulated caches and predictor index by address, so layouts of one
# source (v0.25.1) moved instruction-L1 misses by up to 133% and indirect
# mispredictions by up to 475%, and an edit inside one function turned 970
# indirect mispredictions into 859k with equal instruction counts. The
# hardware branch-misses counter is reported only as well: its run-to-run
# spread is small, but it depends on branch addresses too. On identical
# instruction counts, a few bytes of padding inside one function moved
# nearby_swap's branch misses +9.6% on four of five layouts while its wall
# time moved -0.3%. Branch and cache behaviour that costs time shows up in
# the graded wall time.
# The timed trials rotate through the same layouts: trial t runs layout
# t mod SIM_LAYOUTS on both sides, so every timed metric (wall time, hardware
# instructions, the reported hardware counters) is a median over all layouts
# rather than the reading of one placement. Measured on one layout, identical code moved
# by 32 bytes read +10% wall time (ruin) and +9.9% hardware branch misses
# (nearby_swap), while the medians over five layouts moved +1.2% and +1.0%.
SIM_LAYOUTS=${SIM_LAYOUTS:-5}
SIM_JOBS=${SIM_JOBS:-8}
# Each trial's hardware counters are the mean of PERF_REPEATS process runs.
# cache-misses counts last-level misses, and the last-level cache is shared
# with every other core, so one run's count spreads by 20-80% of its median
# even on an isolated CPU. Averaging PERF_REPEATS runs per trial narrows that
# spread enough for the regression budget to separate a change from noise.
PERF_REPEATS=${PERF_REPEATS:-45}
MEASURE_ITERATIONS=${MEASURE_ITERATIONS:-32}
CARTESIAN_MEASURE_ITERATIONS=${CARTESIAN_MEASURE_ITERATIONS:-256}
# construction_first_fit places one value per entity and stops, so 32
# iterations measure about 0.1 ms of placement against a process start whose
# cache misses (about 2.4k) vary by more than the regression budget. 1024
# iterations make the measured placement work dominate every counter.
FIRST_FIT_MEASURE_ITERATIONS=${FIRST_FIT_MEASURE_ITERATIONS:-1024}
# The full list neighborhoods (list_change, list_swap, sublist_change,
# sublist_swap and their union) enumerate 40k-130k candidates per iteration
# through the compiled cursors, 10-40 ms each. Four iterations keep a trial
# at 40-160 ms like the other cases, so each trial still measures hundreds of
# millions of instructions while 51 trials x 45 counter repeats finish in
# hours rather than a day.
FULL_LIST_MEASURE_ITERATIONS=${FULL_LIST_MEASURE_ITERATIONS:-4}
TOOLCHAIN=${TOOLCHAIN:-1.95.0}
RESULTS=${RESULTS:-"/tmp/solverforge-zero-regression-$(date +%Y%m%d-%H%M%S)"}
# Opt-in scoring cases (not in the default selector set): BENCH_CASES="scoring
# scoring_weights_one scoring_weights_mixed" measures incremental score-director
# churn through uni, join, shared grouped, and balance constraints; see
# crates/solverforge-solver/benches/selector_cursor_gate/scoring_cases.rs.
DEFAULT_CASES=(list_change list_swap nearby_change nearby_swap sublist_change sublist_swap union cartesian k_opt ruin scalar_change scalar_swap construction_full construction_first_fit)
if [[ -n ${BENCH_CASES:-} ]]; then
    read -r -a CASES <<<"$BENCH_CASES"
else
    CASES=("${DEFAULT_CASES[@]}")
fi

if [[ $BENCH_ISOLATE != 0 && $BENCH_ISOLATE != 1 ]]; then
    echo "BENCH_ISOLATE must be 0 or 1" >&2
    exit 2
fi
if [[ $BENCH_ISOLATE == 1 ]] && ! sudo -n true 2>/dev/null; then
    echo "BENCH_ISOLATE=1 needs passwordless sudo to isolate CPU ${BENCH_CPU}" >&2
    exit 2
fi
if ! [[ $SIM_LAYOUTS =~ ^[1-9][0-9]*$ && $SIM_JOBS =~ ^[1-9][0-9]*$ ]]; then
    echo "SIM_LAYOUTS and SIM_JOBS must be positive integers" >&2
    exit 2
fi
if [[ $BASELINE_CURSOR_API != 0 && $BASELINE_CURSOR_API != 1 ]]; then
    echo "BASELINE_CURSOR_API must be 0 or 1" >&2
    exit 2
fi
BENCH_SOURCE="$ROOT/crates/solverforge-solver/benches/selector_cursor_gate.rs"
if [[ $BENCH_OWN_SOURCES != 0 && $BENCH_OWN_SOURCES != 1 ]]; then
    echo "BENCH_OWN_SOURCES must be 0 or 1" >&2
    exit 2
fi

# Both sides run from paths of equal length. The kernel copies the executable
# path onto the new process's stack, so the path length shifts every stack
# address and with it cache behaviour: one list_swap binary measured about
# 260k cache-references from 88-character paths and about 295k from
# 89-character ones, and the old baseline-<case>/candidate-<case> names put the
# two sides on either side of that step.
case_binary() {
    local side=$1
    local case_name=$2
    local layout=${3:-0}
    case $side in
        baseline) printf '%s\n' "$RESULTS/binaries/base/l$layout/$case_name" ;;
        candidate) printf '%s\n' "$RESULTS/binaries/cand/l$layout/$case_name" ;;
        *)
            echo "unknown side: $side" >&2
            exit 2
            ;;
    esac
}

measure_iterations_for_case() {
    local case_name=$1
    if [[ $case_name == cartesian ]]; then
        printf '%s\n' "$CARTESIAN_MEASURE_ITERATIONS"
    elif [[ $case_name == construction_first_fit ]]; then
        printf '%s\n' "$FIRST_FIT_MEASURE_ITERATIONS"
    elif [[ $case_name =~ ^(list_change|list_swap|sublist_change|sublist_swap|union)$ ]]; then
        printf '%s\n' "$FULL_LIST_MEASURE_ITERATIONS"
    else
        printf '%s\n' "$MEASURE_ITERATIONS"
    fi
}

if ! command -v taskset >/dev/null; then
    echo "taskset is required" >&2
    exit 2
fi
if ! command -v objcopy >/dev/null; then
    echo "objcopy is required" >&2
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

mkdir -p "$RESULTS/raw" "$RESULTS/harnesses" "$RESULTS/targets" "$RESULTS/binaries" "$RESULTS/trees"
# Both sides are staged into equal-length sibling paths. Path dependencies
# embed their paths in the binary; unequal path lengths shift string constants
# and code layout, which alone can move hardware counters by several percent
# between identical sources.
BASELINE_TREE="$RESULTS/trees/base"
CANDIDATE_TREE="$RESULTS/trees/cand"
BASELINE_REGISTERED=0
ISOLATION_CGROUP=""
cleanup() {
    if [[ -n $ISOLATION_CGROUP ]]; then
        echo $$ | sudo -n tee /sys/fs/cgroup/cgroup.procs >/dev/null 2>&1 || true
        sudo -n rmdir "$ISOLATION_CGROUP" 2>/dev/null || true
    fi
    if [[ $BASELINE_REGISTERED == 1 ]]; then
        git -C "$ROOT" worktree remove --force "$BASELINE_TREE" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

BASELINE_COMMIT=$(git -C "$ROOT" rev-parse "${BASELINE}^{commit}")
git -C "$ROOT" worktree add --detach "$BASELINE_TREE" "$BASELINE_COMMIT" >/dev/null
BASELINE_REGISTERED=1
# A committed tree may carry a gate bench that predates an API cleanup (benches
# are not compiled by the regular build, so a broken bench can ship unnoticed).
# A fixup only applies to the API generation it was written for; when the
# baseline already contains the repaired bench the patch reverses and is
# skipped, so newer baselines stay measurable instead of aborting the gate.
for fixup in scripts/bench/*-fixup.patch; do
    [[ -e $fixup ]] || continue
    if patch -p1 -d "$BASELINE_TREE" --dry-run --silent <"$fixup" >/dev/null 2>&1; then
        patch -p1 -d "$BASELINE_TREE" --silent <"$fixup" || {
            echo "bench fixup $fixup did not apply cleanly" >&2
            exit 2
        }
    else
        echo "bench fixup $fixup not applicable to ${BASELINE}; skipped" >&2
    fi
done
# Candidate side: the current working tree (tracked + untracked, not ignored),
# so a dirty tree can still be measured, against the same-length path.
mkdir -p "$CANDIDATE_TREE"
(
    cd "$ROOT"
    git ls-files -co --exclude-standard -z |
        tar --null -T - -cf - | (cd "$CANDIDATE_TREE" && tar xf -)
)

make_harness() {
    local side=$1
    local tree=$2
    local directory="$RESULTS/harnesses/$side"
    mkdir -p "$directory"
    local bench_source="$BENCH_SOURCE"
    if [[ $BENCH_OWN_SOURCES == 1 ]]; then
        bench_source="$tree/crates/solverforge-solver/benches/selector_cursor_gate.rs"
    fi
    cat >"$directory/Cargo.toml" <<EOF
[package]
name = "solverforge-selector-gate"
version = "0.0.0"
edition = "2021"
publish = false

[features]
candidate = []
# Enables the scoring cases' runtime constraint-weight variants; set on both
# sides of scoring_weights_* cases only, so no other case links the extra code.
constraint_weights = []

[[bin]]
name = "selector_cursor_gate"
path = "$bench_source"

[dependencies]
solverforge-config = { path = "$tree/crates/solverforge-config" }
solverforge-core = { path = "$tree/crates/solverforge-core" }
solverforge-scoring = { path = "$tree/crates/solverforge-scoring" }
solverforge-solver = { path = "$tree/crates/solverforge-solver" }

[profile.release]
codegen-units = 1
incremental = false
EOF
    cargo "+$TOOLCHAIN" generate-lockfile --manifest-path "$directory/Cargo.toml" >/dev/null
}

# Both sides share one harness package name so identical sources produce
# identical symbol mangling and .text layout; the sides differ only by which
# dependency tree each harness points at.
make_harness baseline "$BASELINE_TREE"
make_harness candidate "$CANDIDATE_TREE"

for case_name in "${CASES[@]}"; do
    if [[ $case_name == scoring_weights_* ]] &&
        ! grep -rqs 'pub struct ConstraintWeights' "$BASELINE_TREE/crates/solverforge-core/src"; then
        echo "$case_name needs a baseline with runtime constraint weights; ${BASELINE} has none" >&2
        exit 2
    fi
done

build_case_binary() {
    local side=$1
    local case_name=$2
    local layout=$3
    local manifest="$RESULTS/harnesses/$side/Cargo.toml"
    local target="$RESULTS/targets/$side/l$layout"
    local rustflags=${RUSTFLAGS:-}
    if ((layout > 0)); then
        rustflags="$rustflags -C metadata=sf-layout-$layout"
    fi
    local feature_args=()
    if [[ $case_name == scoring_weights_* ]]; then
        # Both sides score with the same weights, so the case compares the
        # two trees' weighted scoring rather than weighted against unweighted.
        # Only these cases compile the weight API.
        feature_args=(--features "candidate constraint_weights")
    elif [[ $side == candidate || $BASELINE_CURSOR_API == 1 ]]; then
        feature_args=(--features candidate)
    fi
    SOLVERFORGE_BENCH_CASE="$case_name" CARGO_TARGET_DIR="$target" RUSTFLAGS="$rustflags" \
        cargo "+$TOOLCHAIN" build --manifest-path "$manifest" --release --locked \
        "${feature_args[@]}" >/dev/null
    local binary
    binary=$(case_binary "$side" "$case_name" "$layout")
    mkdir -p "$(dirname "$binary")"
    cp "$target/release/selector_cursor_gate" "$binary"
}

for case_name in "${CASES[@]}"; do
    for ((layout = 0; layout < SIM_LAYOUTS; layout++)); do
        build_case_binary baseline "$case_name" "$layout"
        build_case_binary candidate "$case_name" "$layout"
    done
done

command -v valgrind >/dev/null || {
    echo "valgrind is required for the simulated cache and branch metrics" >&2
    exit 2
}
# Simulated counts do not depend on machine load, so the simulations run
# before the timed trials, SIM_JOBS at a time, outside the isolated CPU.
simulate() {
    local side=$1
    local case_name=$2
    local layout=$3
    local case_iterations
    case_iterations=$(measure_iterations_for_case "$case_name")
    setarch -R valgrind --tool=cachegrind --cache-sim=yes --branch-sim=yes \
        --cachegrind-out-file="$RESULTS/raw/${side}-${case_name}-l${layout}.cachegrind" \
        "$(case_binary "$side" "$case_name" "$layout")" "$case_name" "$case_iterations" \
        >/dev/null 2>"$RESULTS/raw/${side}-${case_name}-l${layout}.cachegrind.log"
}
running=0
for case_name in "${CASES[@]}"; do
    for ((layout = 0; layout < SIM_LAYOUTS; layout++)); do
        for side in baseline candidate; do
            simulate "$side" "$case_name" "$layout" &
            running=$((running + 1))
            if ((running >= SIM_JOBS)); then
                wait -n
                running=$((running - 1))
            fi
        done
    done
done
wait

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

ISOLATED_CPUS=none
if [[ $BENCH_ISOLATE == 1 ]]; then
    ISOLATED_CPUS=$SIBLINGS
    if [[ $ISOLATED_CPUS == unknown ]]; then
        ISOLATED_CPUS=$BENCH_CPU
    fi
    # Builds are done; only measurement runs from here on, so the script
    # itself moves into the partition and every trial inherits it.
    ISOLATION_CGROUP=/sys/fs/cgroup/solverforge-bench-$$
    sudo -n mkdir "$ISOLATION_CGROUP"
    echo "$ISOLATED_CPUS" | sudo -n tee "$ISOLATION_CGROUP/cpuset.cpus" >/dev/null
    echo isolated | sudo -n tee "$ISOLATION_CGROUP/cpuset.cpus.partition" >/dev/null
    if [[ $(<"$ISOLATION_CGROUP/cpuset.cpus.partition") != isolated ]]; then
        echo "could not isolate CPUs ${ISOLATED_CPUS}: $(<"$ISOLATION_CGROUP/cpuset.cpus.partition")" >&2
        exit 2
    fi
    echo $$ | sudo -n tee "$ISOLATION_CGROUP/cgroup.procs" >/dev/null
fi

cat >"$RESULTS/environment.json" <<EOF
{"baseline_requested":"$BASELINE","baseline_commit":"$BASELINE_COMMIT","baseline_cursor_api":$BASELINE_CURSOR_API,"candidate_head":"$(git -C "$ROOT" rev-parse HEAD)","candidate_dirty":$CANDIDATE_DIRTY,"candidate_tree_sha256":"$CANDIDATE_TREE_SHA256","profile":"release(codegen-units=1,incremental=false)","case_isolated_binaries":true,"cpu_model":"$CPU_MODEL","affinity_cpu":$BENCH_CPU,"thread_siblings":"$SIBLINGS","governor":"$GOVERNOR","toolchain":"$TOOLCHAIN","warmups":$WARMUPS,"trials":$TRIALS,"measure_iterations":{"default":$MEASURE_ITERATIONS,"cartesian":$CARTESIAN_MEASURE_ITERATIONS,"construction_first_fit":$FIRST_FIT_MEASURE_ITERATIONS,"full_list":$FULL_LIST_MEASURE_ITERATIONS},"perf_available":$PERF_AVAILABLE,"isolated_cpus":"$ISOLATED_CPUS","perf_repeats":$PERF_REPEATS,"sim_layouts":$SIM_LAYOUTS,"tolerance_percent":$BENCH_TOLERANCE_PERCENT}
EOF

for case_name in "${CASES[@]}"; do
    case_iterations=$(measure_iterations_for_case "$case_name")
    for ((warmup = 0; warmup < WARMUPS; warmup++)); do
        layout=$((warmup % SIM_LAYOUTS))
        taskset -c "$BENCH_CPU" "$(case_binary baseline "$case_name" "$layout")" "$case_name" "$case_iterations" >/dev/null
        taskset -c "$BENCH_CPU" "$(case_binary candidate "$case_name" "$layout")" "$case_name" "$case_iterations" >/dev/null
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
        perf stat -r "$PERF_REPEATS" -x, -o "$perf_file" \
            -e instructions:u,cycles:u,branches:u,branch-misses:u,cache-references:u,cache-misses:u -- \
            taskset -c "$BENCH_CPU" "$binary" "$case_name" "$case_iterations" >/dev/null
    fi
    tr -d '\n' <"$sample_file" >>"$json_file"
    printf '\n' >>"$json_file"
    tr -d '[:space:]' <"$sample_rss" >>"$rss_file"
    printf '\n' >>"$rss_file"
}

for case_name in "${CASES[@]}"; do
    for ((trial = 0; trial < TRIALS; trial++)); do
        layout=$((trial % SIM_LAYOUTS))
        BASELINE_BIN=$(case_binary baseline "$case_name" "$layout")
        CANDIDATE_BIN=$(case_binary candidate "$case_name" "$layout")
        if ((trial % 2 == 0)); then
            run_trial baseline "$BASELINE_BIN" "$case_name" "$trial"
            run_trial candidate "$CANDIDATE_BIN" "$case_name" "$trial"
        else
            run_trial candidate "$CANDIDATE_BIN" "$case_name" "$trial"
            run_trial baseline "$BASELINE_BIN" "$case_name" "$trial"
        fi
    done
done

python3 - "$RESULTS" "$PERF_AVAILABLE" "$BENCH_TOLERANCE_PERCENT" "$BENCH_OWN_SOURCES" "$SIM_LAYOUTS" "${CASES[@]}" <<'PY' 
import hashlib
import json
import math
import pathlib
import random
import statistics
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
perf_available = sys.argv[2] == "1"
tolerance_percent = sys.argv[3]
own_sources = sys.argv[4] == "1"
sim_layouts = int(sys.argv[5])
cases = sys.argv[6:]
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

# Hardware counters reported for diagnosis only: like the simulated cache and
# branch metrics, they depend on code and data addresses as well as on work.
reported_only = {"cycles", "branches", "branch-misses", "cache-references", "cache-misses"}
# Simulated metrics that move with code placement alone; reported only.
simulated_reported_only = {"sim_i1_misses", "sim_branch_mispredicts"}

def load_cachegrind_layout(side, case, layout):
    path = root / "raw" / f"{side}-{case}-l{layout}.cachegrind"
    events = summary = None
    for line in path.read_text().splitlines():
        if line.startswith("events:"):
            events = line.split()[1:]
        elif line.startswith("summary:"):
            summary = [int(value) for value in line.split()[1:]]
    if events is None or summary is None or len(events) != len(summary):
        raise SystemExit(f"unreadable Cachegrind output for {side} {case}")
    totals = dict(zip(events, summary))
    return {
        "sim_instructions": totals["Ir"],
        "sim_d1_misses": totals["D1mr"] + totals["D1mw"],
        "sim_ll_misses": totals["ILmr"] + totals["DLmr"] + totals["DLmw"],
        "sim_i1_misses": totals["I1mr"],
        "sim_branch_mispredicts": totals["Bcm"] + totals["Bim"],
    }

# Each simulated metric is graded on its median across the side's layouts
# (builds that differ only in symbol hashes), so function placement luck does
# not decide the verdict; every layout's count stays in the report.
def load_cachegrind(side, case):
    layouts = [load_cachegrind_layout(side, case, layout) for layout in range(sim_layouts)]
    return {
        metric: (statistics.median(values[metric] for values in layouts), [values[metric] for values in layouts])
        for metric in layouts[0]
    }

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

def text_sha256(binary):
    dump = pathlib.Path(str(binary) + ".text")
    with dump.open("wb") as handle:
        subprocess.run(
            ["objcopy", "--dump-section", f".text={dump}", str(binary)],
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return hashlib.sha256(dump.read_bytes()).hexdigest()


failed = False
report = {"cases": {}, "passed": True}
for case_index, case in enumerate(cases):
    baseline = load_json("baseline", case)
    candidate = load_json("candidate", case)
    if len(baseline) != len(candidate):
        raise SystemExit(f"unaligned samples for {case}")
    baseline_identity = {sample.get("identity_hash", 0) for sample in baseline}
    candidate_identity = {sample.get("identity_hash", 0) for sample in candidate}
    if own_sources:
        # Own-source runs span API generations: hashes are not comparable
        # across trees. The semantic contract is exact-count parity.
        baseline_count = {sample["candidate_count"] for sample in baseline}
        candidate_count = {sample["candidate_count"] for sample in candidate}
        semantic_match = (
            len(baseline_count) == 1
            and baseline_count == candidate_count
        )
    elif case == "construction_first_fit":
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
        # Cross-tree: candidate counts and the encoding-insensitive order hash
        # (raw coordinates/values). Per-tree: the identity hash must be a
        # singleton on both sides (trial-to-trial determinism); it is not
        # compared across trees, because a tabu-identity encoding migration
        # moves it by design.
        baseline_order = {sample["order_hash"] for sample in baseline}
        candidate_order = {sample["order_hash"] for sample in candidate}
        baseline_count = {sample["candidate_count"] for sample in baseline}
        candidate_count = {sample["candidate_count"] for sample in candidate}
        semantic_match = (
            len(baseline_identity) == 1
            and len(candidate_identity) == 1
            and baseline_order == candidate_order
            and len(baseline_order) == 1
            and baseline_count == candidate_count
            and len(baseline_count) == 1
        )
    case_report = {"semantic_match": semantic_match, "metrics": {}}
    if own_sources:
        case_report["candidate_identity"] = {
            "candidate_count": sorted(candidate_count)[0],
        }
    elif case == "construction_first_fit":
        case_report["generated_work"] = {
            "baseline": next(iter(baseline_work)),
            "candidate": next(iter(candidate_work)),
        }
        case_report["selected_trace_hash"] = sorted(candidate_order)[0]
    else:
        case_report["candidate_identity"] = {
            "candidate_count": sorted(candidate_count)[0],
            "order_hash": sorted(candidate_order)[0],
            "identity_hash": sorted(candidate_identity)[0],
        }
    failed |= not semantic_match

    case_report = {
        "semantic_match": semantic_match,
        "text_sha256": {
            "baseline": text_sha256(root / "binaries" / "base" / "l0" / case),
            "candidate": text_sha256(root / "binaries" / "cand" / "l0" / case),
        },
        "metrics": {},
    }

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

    simulated_baseline = load_cachegrind("baseline", case)
    simulated_candidate = load_cachegrind("candidate", case)
    for metric, (before, baseline_layouts) in simulated_baseline.items():
        after, candidate_layouts = simulated_candidate[metric]
        delta_percent = (after - before) * 100.0 / before if before else 0.0
        if metric in simulated_reported_only:
            passed = True
            criterion = "reported"
        else:
            passed = after <= before if before == 0 else delta_percent <= float(tolerance_percent)
            criterion = f"simulated_layout_median_budget_{float(tolerance_percent):g}_percent"
        failed |= not passed
        case_report["metrics"][metric] = {
            "baseline": before,
            "candidate": after,
            "baseline_layouts": baseline_layouts,
            "candidate_layouts": candidate_layouts,
            "delta_percent": delta_percent,
            "criterion": criterion,
            "passed": passed,
        }

    # Deterministic counters must match exactly; any extra allocation,
    # allocated byte, or live byte is a regression. Wall time and hardware
    # instructions are graded against the documented median regression budget
    # (BENCH_TOLERANCE_PERCENT); the remaining hardware counters are reported
    # only, because they move with code placement as well as with work. The bootstrap upper bound stays in the report as a noise
    # diagnostic, not an acceptance criterion.
    exact_metrics = {"allocations", "allocated_bytes", "peak_live_bytes"}
    budget = float(tolerance_percent)
    for metric_index, metric in enumerate(metrics_for_case):
        baseline_values = [sample[metric] for sample in baseline]
        candidate_values = [sample[metric] for sample in candidate]
        differences = [after - before for before, after in zip(baseline_values, candidate_values)]
        baseline_median = statistics.median(baseline_values)
        candidate_median = statistics.median(candidate_values)
        upper = bootstrap_upper(differences, case_index * 100 + metric_index)
        delta_percent = (
            (candidate_median - baseline_median) * 100.0 / baseline_median
            if baseline_median
            else 0.0
        )
        if metric in reported_only:
            passed = True
            criterion = "reported"
        elif metric in exact_metrics or baseline_median == 0:
            passed = candidate_median <= baseline_median
            criterion = "exact"
        else:
            passed = delta_percent <= budget
            criterion = f"median_budget_{budget:g}_percent"
        failed |= not passed
        case_report["metrics"][metric] = {
            "baseline_median": baseline_median,
            "candidate_median": candidate_median,
            "baseline_iqr": interquartile_range(baseline_values),
            "candidate_iqr": interquartile_range(candidate_values),
            "delta_percent": delta_percent,
            "paired_median_difference": statistics.median(differences),
            "paired_bootstrap_upper_95": upper,
            "criterion": criterion,
            "passed": passed,
        }
    report["cases"][case] = case_report

report["passed"] = not failed
(root / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
print(json.dumps(report, indent=2, sort_keys=True))
raise SystemExit(1 if failed else 0)
PY

echo "zero-regression evidence: $RESULTS"
