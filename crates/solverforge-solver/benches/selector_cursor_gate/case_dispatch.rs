/* The zero-regression gate builds one executable per case and passes the case
name at compile time. A case is linked only into its own executable: every
other case's body is compiled out, so unrelated case code cannot shift the
measured case's layout and perturb branch-predictor and cache counters.
Without the variable, one executable holds every case and picks at runtime. */
const BUILD_CASE: Option<&str> = option_env!("SOLVERFORGE_BENCH_CASE");

const fn same_str(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn linked(name: &str) -> bool {
    match BUILD_CASE {
        Some(case) => same_str(case, name),
        None => true,
    }
}

macro_rules! run_case {
    ($case:expr, $matched:ident, $name:literal, $function:path) => {
        if const { linked($name) } && ($case == "all" || $case == $name) {
            $matched = true;
            $function();
        }
    };
}
