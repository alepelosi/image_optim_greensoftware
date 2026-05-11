use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

/// Some function we want to benchmark.
fn factorial(x: u128) -> Result<u128, ()> {
    let res = (1..x).fold(1, u128::saturating_mul);
    Ok(res)
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

    for size in [100_000, 1000_000, 10_000_000] {
        bench.benchmark(size, &|| factorial(size));
    }
}
