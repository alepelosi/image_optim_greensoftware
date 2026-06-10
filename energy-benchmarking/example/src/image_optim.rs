use std::process::Command;
use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

const REPO_PATH: &str = "/home/alessandro/Desktop/Uni/GreenSoftware/optimization1";
const BENCH_DIR: &str = "runs/bench";

// Empty means: use all installed/relevant optimizers in the full setup.
const BASE_FLAGS: &[&str] = &[];

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

fn run_command_print(args: &[&str]) -> Result<(), String> {
    let output = Command::new(args[0])
        .current_dir(REPO_PATH)
        .args(&args[1..])
        .output()
        .map_err(|e| format!("Failed to start command {:?}: {e}", args))?;

    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("Command {:?} failed", args))
    }
}

fn check_environment() -> Result<(), String> {
    println!("Checking benchmark environment...");

    run_command_print(&["ruby", "--version"])?;
    run_command_print(&["bundle", "--version"])?;
    run_command_print(&["ls", "-lh", "./bin/image_optim"])?;

    println!("Checking datasets...");
    run_command_print(&["bash", "-lc", "echo -n 'test_images: '; find test_images -type f | wc -l"])?;
    run_command_print(&["bash", "-lc", "echo -n 'black_dataset: '; find black_dataset -type f | wc -l"])?;
    run_command_print(&["bash", "-lc", "echo -n 'white_dataset: '; find white_dataset -type f | wc -l"])?;
    run_command_print(&["bash", "-lc", "echo -n 'noisy_gifs: '; find noisy_gifs -type f | wc -l"])?;

    println!("Checking available optimizers...");
    run_command_print(&["bash", "-lc", "echo -n 'svgo: '; command -v svgo || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'pngout: '; command -v pngout || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'optipng: '; command -v optipng || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'pngcrush: '; command -v pngcrush || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'advpng: '; command -v advpng || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'pngquant: '; command -v pngquant || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'jpegoptim: '; command -v jpegoptim || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'jpegtran: '; command -v jpegtran || true"])?;
    run_command_print(&["bash", "-lc", "echo -n 'gifsicle: '; command -v gifsicle || true"])?;

    println!("Environment check completed.");
    Ok(())
}

fn prepare_input(dataset_dir: &str) {
    // SETUP PHASE: not part of the measured benchmark when using benchmark_with.
    run_command(&["rm", "-rf", BENCH_DIR])
        .expect("Failed to remove old benchmark directory");

    run_command(&["mkdir", "-p", "runs"])
        .expect("Failed to create runs directory");

    run_command(&["cp", "-r", dataset_dir, BENCH_DIR])
        .expect("Failed to copy benchmark dataset");
}

fn run_image_optim(extra_flags: &[&str]) -> Result<(), String> {
    // MEASURED PHASE: only image_optim should be measured.
    let mut args = vec![
        "bundle",
        "exec",
        "./bin/image_optim",
        "-r",
        BENCH_DIR,
    ];

    args.extend(BASE_FLAGS);
    args.extend(extra_flags);

    run_command(&args)
}

fn cleanup_input() {
    // CLEANUP PHASE: not part of the measured benchmark when using benchmark_with.
    let _ = run_command(&["rm", "-rf", BENCH_DIR]);
}

fn bench_image_dataset(config: EnergyBenchConfig, bench_name: &str, dataset_dir: &'static str) {
    let mut bench = EnergyBench::new(bench_name, config);

    bench.benchmark_with(
        "full_all_optimizers",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&[]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_svgo",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-svgo"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_pngout",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-pngout"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_optipng",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-optipng"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_pngcrush",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-pngcrush"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_advpng",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-advpng"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_pngquant",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-pngquant"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_jpegoptim",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-jpegoptim"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_jpegtran",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-jpegtran"]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_gifsicle",
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(&["--no-gifsicle"]),
        &|_| cleanup_input(),
    );
}

fn bench_gif_dataset(config: EnergyBenchConfig) {
    let mut bench = EnergyBench::new("image_optim_noisy_gifs_measurements", config);

    bench.benchmark_with(
        "full_all_optimizers",
        &|| prepare_input("noisy_gifs"),
        &|_| run_image_optim(&[]),
        &|_| cleanup_input(),
    );

    bench.benchmark_with(
        "no_gifsicle",
        &|| prepare_input("noisy_gifs"),
        &|_| run_image_optim(&["--no-gifsicle"]),
        &|_| cleanup_input(),
    );
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

    bench_image_dataset(
        config.clone(),
        "image_optim_test_images_measurements",
        "test_images",
    );

    bench_image_dataset(
        config.clone(),
        "image_optim_black_dataset_measurements",
        "black_dataset",
    );

    bench_image_dataset(
        config.clone(),
        "image_optim_white_dataset_measurements",
        "white_dataset",
    );

    bench_gif_dataset(config);
}