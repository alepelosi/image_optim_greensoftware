use clap::Parser;
use energy_bench::{EnergyBench, EnergyBenchConfig};
use std::io;
use std::ops;
use std::process::{Child, Command, Stdio};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = String::from("energy-bench"))]
    filename: String,
    #[arg(long)]
    benchmark_name: Option<String>,
    #[command(flatten)]
    config: EnergyBenchConfig,
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

fn run(mut cmd: Command) -> io::Result<Command> {
    ChildGuard(cmd.spawn()?).wait()?;
    Ok(cmd)
}

struct Metadata(String);

impl energy_bench::Metadata<1> for Metadata {
    fn get_header() -> [&'static str; 1] {
        ["cmd"]
    }

    fn get_values(&self) -> [String; 1] {
        [self.0.clone()]
    }
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    let Args {
        filename,
        benchmark_name,
        config,
        args,
    } = Args::parse();

    let mut bench = EnergyBench::new(&filename, config);

    let (prog, args) = args.split_first().expect("no command provided");
    let make_cmd = || {
        let mut cmd = Command::new(prog);
        cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
        cmd
    };

    bench.benchmark_with(
        Metadata(benchmark_name.unwrap_or(prog.clone())),
        &make_cmd,
        &run,
        &|_| (),
    );
}

/// Ensures that the spawned process is killed, even if the benchmark is canceled.
struct ChildGuard(Child);

impl ops::Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
