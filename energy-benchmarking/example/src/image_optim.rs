use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use energy_bench::{EnergyBench, EnergyBenchConfig};

const REPO_PATH: &str = "/home/alessandro/Desktop/Uni/GreenSoftware/optimization1";
const BENCH_DIR: &str = "runs/bench";
const SIZE_CSV: &str = "runs/image_optim_size_reductions.csv";

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
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'test_images: '; find test_images -type f | wc -l",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'black_dataset: '; find black_dataset -type f | wc -l",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'white_dataset: '; find white_dataset -type f | wc -l",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'noisy_gifs: '; find noisy_gifs -type f | wc -l",
    ])?;

    println!("Checking available optimizers...");
    run_command_print(&["bash", "-lc", "echo -n 'svgo: '; command -v svgo || true"])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'pngout: '; command -v pngout || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'optipng: '; command -v optipng || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'pngcrush: '; command -v pngcrush || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'advpng: '; command -v advpng || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'pngquant: '; command -v pngquant || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'jpegoptim: '; command -v jpegoptim || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'jpegtran: '; command -v jpegtran || true",
    ])?;
    run_command_print(&[
        "bash",
        "-lc",
        "echo -n 'gifsicle: '; command -v gifsicle || true",
    ])?;

    println!("Environment check completed.");
    Ok(())
}

fn prepare_input(dataset_dir: &str) {
    // SETUP PHASE: not part of the measured benchmark when using benchmark_with.
    run_command(&["rm", "-rf", BENCH_DIR]).expect("Failed to remove old benchmark directory");

    run_command(&["mkdir", "-p", "runs"]).expect("Failed to create runs directory");

    run_command(&["cp", "-r", dataset_dir, BENCH_DIR]).expect("Failed to copy benchmark dataset");
}

fn run_image_optim(extra_flags: &[&str]) -> Result<(), String> {
    // MEASURED PHASE: only image_optim should be measured.
    let mut args = vec!["bundle", "exec", "./bin/image_optim", "-r", BENCH_DIR];

    args.extend(BASE_FLAGS);
    args.extend(extra_flags);

    run_command(&args)
}

fn cleanup_input() {
    // CLEANUP PHASE: not part of the measured benchmark when using benchmark_with.
    let _ = run_command(&["rm", "-rf", BENCH_DIR]);
}

fn repo_path(path: &str) -> PathBuf {
    Path::new(REPO_PATH).join(path)
}

fn directory_size_and_count(path: &str) -> Result<(u64, u64), String> {
    fn visit(path: &Path, total_size: &mut u64, file_count: &mut u64) -> Result<(), String> {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory {}: {e}", path.display()))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
            let entry_path = entry.path();
            let metadata = entry.metadata().map_err(|e| {
                format!("Failed to read metadata for {}: {e}", entry_path.display())
            })?;

            if metadata.is_dir() {
                visit(&entry_path, total_size, file_count)?;
            } else if metadata.is_file() {
                *total_size += metadata.len();
                *file_count += 1;
            }
        }

        Ok(())
    }

    let mut total_size = 0;
    let mut file_count = 0;
    visit(&repo_path(path), &mut total_size, &mut file_count)?;
    Ok((total_size, file_count))
}

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn append_csv_record(path: &str, fields: &[String]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(repo_path(path))
        .map_err(|e| format!("Failed to open CSV {path}: {e}"))?;

    let row = fields
        .iter()
        .map(|field| csv_field(field))
        .collect::<Vec<_>>()
        .join(",");

    writeln!(file, "{row}").map_err(|e| format!("Failed to write CSV row to {path}: {e}"))
}

fn init_size_csv() -> Result<(), String> {
    fs::create_dir_all(repo_path("runs"))
        .map_err(|e| format!("Failed to create runs directory for size CSV: {e}"))?;

    let mut file = File::create(repo_path(SIZE_CSV))
        .map_err(|e| format!("Failed to create size CSV {SIZE_CSV}: {e}"))?;

    writeln!(
        file,
        "bench_name,dataset,variant,status,file_count,original_size_bytes,optimized_size_bytes,reduced_bytes,reduced_kb,reduction_percent,error"
    )
    .map_err(|e| format!("Failed to write size CSV header: {e}"))
}

fn append_size_result(
    bench_name: &str,
    dataset_dir: &str,
    variant_name: &str,
    status: &str,
    file_count: u64,
    original_size: u64,
    optimized_size: u64,
    error: &str,
) -> Result<(), String> {
    let reduced_bytes = original_size.saturating_sub(optimized_size);
    let reduced_kb = reduced_bytes as f64 / 1024.0;
    let reduction_percent = if original_size == 0 {
        0.0
    } else {
        reduced_bytes as f64 * 100.0 / original_size as f64
    };

    append_csv_record(
        SIZE_CSV,
        &[
            bench_name.to_string(),
            dataset_dir.to_string(),
            variant_name.to_string(),
            status.to_string(),
            file_count.to_string(),
            original_size.to_string(),
            optimized_size.to_string(),
            reduced_bytes.to_string(),
            format!("{reduced_kb:.3}"),
            format!("{reduction_percent:.3}"),
            error.to_string(),
        ],
    )
}

fn record_size_reduction(
    bench_name: &str,
    dataset_dir: &'static str,
    variant_name: &str,
    extra_flags: &[&str],
) {
    println!("Recording size reduction for {bench_name}/{variant_name}...");

    let result = (|| -> Result<(), String> {
        prepare_input(dataset_dir);

        let (original_size, file_count) = directory_size_and_count(BENCH_DIR)?;

        let status = match run_image_optim(extra_flags) {
            Ok(()) => "ok",
            Err(error) => {
                let (optimized_size, _) =
                    directory_size_and_count(BENCH_DIR).unwrap_or((original_size, file_count));
                append_size_result(
                    bench_name,
                    dataset_dir,
                    variant_name,
                    "failed",
                    file_count,
                    original_size,
                    optimized_size,
                    &error,
                )?;
                return Err(error);
            }
        };

        let (optimized_size, _) = directory_size_and_count(BENCH_DIR)?;
        append_size_result(
            bench_name,
            dataset_dir,
            variant_name,
            status,
            file_count,
            original_size,
            optimized_size,
            "",
        )
    })();

    cleanup_input();

    if let Err(error) = result {
        eprintln!("Failed to record size reduction for {bench_name}/{variant_name}: {error}");
    }
}

fn benchmark_variant(
    bench: &mut EnergyBench<&str, 1>,
    dataset_dir: &'static str,
    variant_name: &'static str,
    extra_flags: &[&str],
) {
    bench.benchmark_with(
        variant_name,
        &|| prepare_input(dataset_dir),
        &|_| run_image_optim(extra_flags),
        &|_| cleanup_input(),
    );
}

fn record_image_dataset_sizes(bench_name: &str, dataset_dir: &'static str) {
    record_size_reduction(bench_name, dataset_dir, "full_all_optimizers", &[]);
    record_size_reduction(bench_name, dataset_dir, "no_svgo", &["--no-svgo"]);
    record_size_reduction(bench_name, dataset_dir, "no_pngout", &["--no-pngout"]);
    record_size_reduction(bench_name, dataset_dir, "no_optipng", &["--no-optipng"]);
    record_size_reduction(bench_name, dataset_dir, "no_pngcrush", &["--no-pngcrush"]);
    record_size_reduction(bench_name, dataset_dir, "no_advpng", &["--no-advpng"]);
    record_size_reduction(bench_name, dataset_dir, "no_pngquant", &["--no-pngquant"]);
    record_size_reduction(
        bench_name,
        dataset_dir,
        "no_jpegoptim",
        &["--no-jpegoptim"],
    );
    record_size_reduction(bench_name, dataset_dir, "no_jpegtran", &["--no-jpegtran"]);
    record_size_reduction(bench_name, dataset_dir, "no_gifsicle", &["--no-gifsicle"]);
}

fn record_gif_dataset_sizes() {
    let bench_name = "image_optim_noisy_gifs_measurements";
    record_size_reduction(bench_name, "noisy_gifs", "full_all_optimizers", &[]);
    record_size_reduction(bench_name, "noisy_gifs", "no_gifsicle", &["--no-gifsicle"]);
}

fn record_all_size_reductions() {
    // SIZE ACCOUNTING PHASE: intentionally after all EnergyBench measurements.
    record_image_dataset_sizes("image_optim_test_images_measurements", "test_images");
    record_image_dataset_sizes("image_optim_black_dataset_measurements", "black_dataset");
    record_image_dataset_sizes("image_optim_white_dataset_measurements", "white_dataset");
    record_gif_dataset_sizes();
}

fn bench_image_dataset(config: EnergyBenchConfig, bench_name: &str, dataset_dir: &'static str) {
    let mut bench = EnergyBench::new(bench_name, config);

    benchmark_variant(&mut bench, dataset_dir, "full_all_optimizers", &[]);

    benchmark_variant(&mut bench, dataset_dir, "no_svgo", &["--no-svgo"]);

    benchmark_variant(&mut bench, dataset_dir, "no_pngout", &["--no-pngout"]);

    benchmark_variant(&mut bench, dataset_dir, "no_optipng", &["--no-optipng"]);

    benchmark_variant(&mut bench, dataset_dir, "no_pngcrush", &["--no-pngcrush"]);

    benchmark_variant(&mut bench, dataset_dir, "no_advpng", &["--no-advpng"]);

    benchmark_variant(&mut bench, dataset_dir, "no_pngquant", &["--no-pngquant"]);

    benchmark_variant(&mut bench, dataset_dir, "no_jpegoptim", &["--no-jpegoptim"]);

    benchmark_variant(&mut bench, dataset_dir, "no_jpegtran", &["--no-jpegtran"]);

    benchmark_variant(&mut bench, dataset_dir, "no_gifsicle", &["--no-gifsicle"]);
}

fn bench_gif_dataset(config: EnergyBenchConfig) {
    let bench_name = "image_optim_noisy_gifs_measurements";
    let mut bench = EnergyBench::new(bench_name, config);

    benchmark_variant(&mut bench, "noisy_gifs", "full_all_optimizers", &[]);

    benchmark_variant(&mut bench, "noisy_gifs", "no_gifsicle", &["--no-gifsicle"]);
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

    init_size_csv().expect("Failed to initialize size-reduction CSV");
    record_all_size_reductions();
}
