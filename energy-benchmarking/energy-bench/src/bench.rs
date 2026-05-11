use std::{
    any,
    fmt,
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    energy_acc::EnergyAccumulator,
    idle::IdlePower,
    metadata::Metadata,
    output::OutputFiles,
    run_result::RunResult,
};

#[derive(clap::Parser, Clone)]
pub struct EnergyBenchConfig {
    /// The number of warmup runs to do.
    ///
    /// These are run before the actual benchmarking process starts, and are often necessary for
    /// multiple reasons. These include: warming up the caches, waiting for turboboost behavior
    /// to subside, and waiting for the CPU to physically heat up.
    /// This may or may not be necessary depending on system configuration and the benchmark.
    #[arg(long, default_value_t = 0)]
    pub warmup_runs: usize,
    /// The number of benchmark runs to do.
    ///
    /// Note that, in combination with `min_run_duration`, a single 'benchmark run' may consist of
    /// multiple 'program runs'. This is necessary for short-running benchmarks, as energy measurements
    /// typically sample at a low frequency and thus require a substantial runtime to be accurate.
    #[arg(long, default_value_t = 1)]
    pub benchmark_runs: usize,
    /// Set the minimal amount of time each benchmark run should take.
    ///
    /// This ensures that each 'benchmark run' takes long enough to be able to accurately measure
    /// energy consumption. This is necessary for short-running benchmarks, as energy measurements
    /// typically sample at a low frequency and thus require a substantial runtime to be accurate.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "500ms")]
    pub min_run_duration: Duration,
    /// Set the time taken to estimate idle power draw.
    #[arg(long, default_value_t = 120)]
    pub idle_duration_seconds: usize,
    /// Set the path to the idle results cache file.
    #[arg(long)]
    pub idle_path: Option<PathBuf>,
}

impl Default for EnergyBenchConfig {
    fn default() -> Self {
        Self {
            warmup_runs: 0,
            benchmark_runs: 1,
            min_run_duration: Duration::from_millis(500),
            idle_duration_seconds: 120,
            idle_path: None,
        }
    }
}

pub struct EnergyBench<MD: Metadata<COLS>, const COLS: usize> {
    warmup_runs: usize,
    benchmark_runs: usize,
    min_run_duration: Duration,
    output: OutputFiles<MD, COLS>,
    idle_power: Option<IdlePower>,
    probes: Box<dyn EnergyAccumulator>,
}

impl<MD: Metadata<COLS>, const COLS: usize> EnergyBench<MD, COLS> {
    pub fn default(name: &str) -> Self {
        let config = EnergyBenchConfig::default();
        Self::new(name, config)
    }

    pub fn new_with_accumulator(name: &str, config: EnergyBenchConfig, mut probes: Box<dyn EnergyAccumulator>) -> Self {
        let idle_power = if config.idle_duration_seconds > 0 {
            Some(IdlePower::init(
                config.idle_path.as_ref(),
                config.idle_duration_seconds,
                &mut probes,
            ))
        } else {
            None
        };

        Self {
            warmup_runs: config.warmup_runs,
            benchmark_runs: config.benchmark_runs,
            min_run_duration: config.min_run_duration,
            output: OutputFiles::new(name),
            idle_power,
            probes,
        }
    }

    pub fn new(name: &str, config: EnergyBenchConfig) -> Self {
        #[cfg(feature = "software-energy-lab")]
        let probes = Box::new(crate::software_energy_lab::SoftwareEnergyLab::new());
        #[cfg(not(feature = "software-energy-lab"))]
        let probes = Box::new(crate::energy_acc::DefaultEnergyAccumulator::new());
        Self::new_with_accumulator(name, config, probes)
    }

    pub fn benchmark<T, E>(&mut self, metadata: MD, bench_fn: &impl Fn() -> Result<T, E>)
    where
        E: fmt::Debug,
    {
        self.warmup(&|| (), &|()| bench_fn().map(|_| ()), &|()| ());

        let measurements = (1..=self.benchmark_runs)
            .into_iter()
            .filter_map(|i| {
                log::trace!("Starting benchmark run {i}");
                match self.benchmark_run(&|| (), &|()| bench_fn().map(|_| ()), &|()| ()) {
                    Ok(measurements) => Some(measurements),
                    Err(e) => {
                        log::error!("Benchmark run {i} failed: {e:?}");
                        None
                    }
                }
            })
            .collect();

        self.write(metadata, measurements);
    }

    pub fn benchmark_with<T, E>(
        &mut self,
        metadata: MD,
        setup_fn: &impl Fn() -> T,
        bench_fn: &impl Fn(T) -> Result<T, E>,
        cleanup_fn: &impl Fn(T),
    ) where
        T: 'static,
        E: fmt::Debug,
    {
        self.warmup(setup_fn, bench_fn, cleanup_fn);

        let measurements = (1..=self.benchmark_runs)
            .into_iter()
            .filter_map(|i| {
                log::trace!("Starting benchmark run {i}");
                match self.benchmark_run(setup_fn, bench_fn, cleanup_fn) {
                    Ok(measurements) => Some(measurements),
                    Err(e) => {
                        log::error!("Benchmark run {i} failed: {e:?}");
                        None
                    }
                }
            })
            .collect();

        self.write(metadata, measurements);
    }

    fn write(&mut self, metadata: MD, measurements: Vec<RunResult>) {
        if measurements.is_empty() {
            log::error!("All benchmarks failed, no results written");
        } else {
            if let Err(e) = self.output.write_results(&metadata, &measurements) {
                log::error!("Error writing results: {}", e);
            }

            if measurements.len() > 1 {
                if let Err(e) = self.output.write_summary(&metadata, &measurements) {
                    log::error!("Error writing results: {}", e);
                }
            }
        }
    }

    fn warmup<T, E>(
        &self,
        setup_fn: &impl Fn() -> T,
        bench_fn: &impl Fn(T) -> Result<T, E>,
        cleanup_fn: &impl Fn(T),
    ) where
        E: fmt::Debug,
    {
        for _ in 0..self.warmup_runs {
            match self.warmup_run(setup_fn, bench_fn, cleanup_fn) {
                Ok(()) => { /* nothing to do */ },
                Err(e) => log::error!("Warmup run failed: {e:?}"),
            }
        }
    }

    fn warmup_run<T, E>(
        &self,
        setup_fn: &impl Fn() -> T,
        bench_fn: &impl Fn(T) -> Result<T, E>,
        cleanup_fn: &impl Fn(T),
    ) -> Result<(), E> where
        E: fmt::Debug,
    {
        let now = Instant::now();

        loop {
            let inp = setup_fn();
            let res = black_box(bench_fn(inp))?;
            cleanup_fn(res);

            if now.elapsed() >= self.min_run_duration {
                break;
            }
        }

        Ok(())
    }

    fn benchmark_run<T, E>(
        &mut self,
        setup_fn: &impl Fn() -> T,
        bench_fn: &impl Fn(T) -> Result<T, E>,
        cleanup_fn: &impl Fn(T),
    ) -> Result<RunResult, E>
    where
        T: 'static,
    {
        let mut run_results = self.measure_run(setup_fn, bench_fn, cleanup_fn)?;

        // If there is no setup or cleanup, i.e. T == (), we assume that there is no overhead
        if any::TypeId::of::<T>() != any::TypeId::of::<()>() {
            let overhead = self.measure_overhead(run_results.repeats, setup_fn, cleanup_fn);
            log::trace!("Overhead: {:?}", overhead);
            run_results.subtract_overhead(overhead);
        }

        if let Some(idle) = &self.idle_power {
            // Overhead has already been subtracted from the runtime at this point
            run_results.subtract_idle(idle);
        }

        // Normalise for number of runs
        run_results.normalise();
        Ok(run_results)
    }

    fn measure_run<T, E>(
        &mut self,
        setup_fn: &impl Fn() -> T,
        bench_fn: &impl Fn(T) -> Result<T, E>,
        cleanup_fn: &impl Fn(T),
    ) -> Result<RunResult, E> {
        self.probes.reset();
        let mut repeats = 0;
        let now = Instant::now();

        loop {
            let inp = setup_fn();
            let res = black_box(bench_fn(inp))?;
            cleanup_fn(res);
            repeats += 1;

            if now.elapsed() >= self.min_run_duration {
                break;
            }
        }

        let runtime = now.elapsed();
        let energy = self.probes.elapsed();
        Ok(RunResult::new(repeats, runtime, energy))
    }

    /// Measure how much of our total consumed energy is due to the data generation. Why do we need
    /// this? The function we are benchmarking might complete very quickly, in which case we must
    /// repeat that function multiple times in between measurements in order to get a non-zero
    /// result. In this case we have no choice but to generate the data whilst measuring as well. So
    /// instead we measure how much overhead was introduced by the data generation after the fact.
    fn measure_overhead<T>(
        &mut self,
        repeats: usize,
        setup_fn: &impl Fn() -> T,
        cleanup_fn: &impl Fn(T),
    ) -> RunResult {
        self.probes.reset();
        let now = Instant::now();

        for _ in 0..repeats {
            cleanup_fn(setup_fn());
        }

        let runtime = now.elapsed();
        let energy = self.probes.elapsed();
        RunResult::new(1, runtime, energy)
    }
}
