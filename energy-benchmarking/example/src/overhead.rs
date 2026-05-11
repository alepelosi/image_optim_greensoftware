use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

fn create_data(len: i32) -> Vec<i32> {
    (0..len).into_iter().map(|x| len - x).collect()
}

/// Some function we want to benchmark.
fn sort(mut xs: Vec<i32>) -> Result<Vec<i32>, ()> {
    xs.sort();
    Ok(xs)
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    let config = EnergyBenchConfig {
        warmup_runs: 1,
        benchmark_runs: 5,
        min_run_duration: Duration::from_millis(100),
        idle_duration_seconds: 30,
        ..EnergyBenchConfig::default()
    };
    let mut bench = EnergyBench::new("options", config);

    bench.benchmark_with("sort",
        &|| create_data(1000),
        &|xs| sort(xs),
        &|_xs| { /* nothing to clean up */ }
    );
}
