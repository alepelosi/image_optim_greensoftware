use std::{hint::black_box, path::PathBuf};

use energy_bench::{EnergyBench, EnergyBenchConfig};

fn start_server() {
    black_box(())
}

fn client() -> Result<(), ()> {
    black_box((1..100_000).fold(1, u128::saturating_add));
    Ok(())
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    start_server();

    // energy-bench automatically calculates idle power draw and caches this value between runs
    // In some cases this is not the wanted behaviour, e.g. when additionally a server is running
    // in the background, which would ideally also be subtracted as idle.
    // For this purpose, the path to the idle file can be changed manually, ensuring that we
    // keep these two 'variants' of idle power draw separate.
    let config = EnergyBenchConfig {
        idle_duration_seconds: 10,
        idle_path: Some(PathBuf::from("./local-idle-cache/")),
        ..Default::default()
    };
    let mut bench = EnergyBench::new("serverclient", config);

    bench.benchmark("client", &client);
}
