use std::process::Command;
use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

fn run_image_optim() -> Result<(), String> {
    let repo = "/Users/alessandropelosi/Desktop/Uni/Master/First Year/Second Semester/Green Software/image_optim_greensoftware";

    let script = r#"
        set -euo pipefail

        echo "Current directory:"
        pwd

        echo "Checking files:"
        ls -la
        ls -la datasets || true
        ls -la datasets/original || true
        ls -la bin/image_optim

        rm -rf runs/bench
        mkdir -p runs
        cp -r test_images runs/bench

        bundle exec bin/image_optim -r runs/bench --no-svgo
    "#;

    let output = Command::new("/bin/bash")
        .current_dir(repo)
        .arg("-lc")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to start shell command: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Err(format!(
            "image_optim failed.\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout, stderr
        ))
    }
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    let config = EnergyBenchConfig {
        warmup_runs: 1,
        benchmark_runs: 10,
        min_run_duration: Duration::from_secs(10),
        idle_duration_seconds: 30,
        ..EnergyBenchConfig::default()
    };

    let mut bench = EnergyBench::new("image_optim", config);

    bench.benchmark("jpg_png_dataset", &|| run_image_optim());
}