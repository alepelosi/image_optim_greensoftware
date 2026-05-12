use std::process::Command;
use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

const REPO_PATH: &str = "/home/artemis/uni/image_optim_greensoftware";

const ORIGINAL_IMAGE: &str = "bin/orig.png";
const BENCH_IMAGE: &str = "bin/t1.png";

const BASE_FLAGS: &[&str] = &[
    "--no-svgo"
];

fn run_command(args: &[&str]) -> Result<(), String> {
    let output = Command::new(args[0])
        .current_dir(REPO_PATH)
        .args(&args[1..])
        .output()
        .map_err(|e| format!("Failed to start command {:?}: {e}", args))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command {:?} failed.\n\nSTDOUT:\n{}\nSTDERR:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn check_environment() -> Result<(), String> {
    run_command(&["ruby", "--version"])?;
    run_command(&["ls", "-lh", "./bin/image_optim"])?;
    run_command(&["ls", "-lh", ORIGINAL_IMAGE])?;

    Ok(())
}

fn prepare_input() -> () {
    // This is setup. With benchmark_with, this part should not be counted
    // as part of the measured benchmark workload.
    run_command(&["rm", "-f", BENCH_IMAGE]);
    run_command(&["cp", ORIGINAL_IMAGE, BENCH_IMAGE]);
}

fn run_image_optim(extra_flags: &[&str]) -> Result<(), String> {
    let mut args = vec!["./bin/image_optim", BENCH_IMAGE];

    args.extend(BASE_FLAGS);
    args.extend(extra_flags);

    run_command(&args)
}

fn run_image_optim_generic() -> Result<(), String> {
    run_image_optim(&[])
}

fn run_image_optim_no_optipng() -> Result<(), String> {
    run_image_optim(&["--no-optipng"])
}

fn run_image_optim_no_pngcrush() -> Result<(), String> {
    run_image_optim(&["--no-pngcrush"])
}

fn run_image_optim_no_advpng() -> Result<(), String> {
    run_image_optim(&["--no-advpng"])
}

fn cleanup_input() {
    // Cleanup should not be part of the measured workload.
    // We ignore errors here because a failed benchmark run may already have removed/changed the file.
    let _ = run_command(&["rm", "-f", BENCH_IMAGE]);
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    check_environment().expect("Environment check failed");

    let config = EnergyBenchConfig {
        warmup_runs: 1,
        benchmark_runs: 10,
        min_run_duration: Duration::from_secs(0),
        idle_duration_seconds: 30,
        ..EnergyBenchConfig::default()
    };

    let mut bench = EnergyBench::new("image_optim_epd_measurements", config);

    bench.benchmark_with(
        "full_png_no_svgo",
        &|| prepare_input(),
        &|_| run_image_optim_generic(),
        &|_result| cleanup_input()
    );

    bench.benchmark_with(
        "no_optipng",
        &|| prepare_input(),
        &|_| run_image_optim_no_optipng(),
        &|_result| cleanup_input()
    );

    bench.benchmark_with(
        "no_pngcrush",
        &|| prepare_input(),
        &|_| run_image_optim_no_pngcrush(),
        &|_result| cleanup_input()
    );

    bench.benchmark_with(
        "no_advpng",
        &|| prepare_input(),
        &|_| run_image_optim_no_advpng(),
        &|_result| cleanup_input()
    );
}
