use std::hint::black_box;

use energy_bench::EnergyBench;

async fn start_server() {
    black_box(())
}

/// Some async function we want to benchmark.
async fn factorial(x: u128) -> Result<u128, ()> {
    let res = (1..x).fold(1, u128::saturating_mul);
    Ok(res)
}

/// Asynchornous functions are spawned in a new thread, which means that if an async function is
/// called from bench.benchmark, all that is being measured is the spawning of the new thread.
/// That job will then continue running in the background, and the benchmark will
/// instantly complete (because all it does is spawn a thread).
///
/// To avoid this we must manually spawn threads and block until they are done.
/// Namely, this means we cannot use the `#[tokio::main]` macro. Instead, we need
/// to manually create a tokio runtime, and manually instrument async functions.
fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    // First, create the tokio runtime. This is what is responsible for creating
    // threads and for optionally blocking until that thread completes,
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // You might still want to spawn a process in the background, like a server
    // For this, use `rt.spawn`, which spawns a new thread without blocking the main thread.
    rt.spawn(start_server());

    let mut bench = EnergyBench::default("minimal");

    for size in [100_000, 1000_000, 10_000_000] {
        // We cannot directly run `bench.benchmark(size, &factorial`, because
        // the factorial will be computed in another thread.
        //
        // Instead, we use rt.block_on to explicitly wait for that thread to complete.
        bench.benchmark(size, &||
            rt.block_on(async {
                factorial(size).await
            })
        );
    }
}
