use energy_bench::EnergyBench;

/// Some function we want to benchmark.
fn factorial(x: u128) -> Result<u128, ()> {
    let res = (1..x).fold(1, u128::saturating_mul);
    Ok(res)
}

struct Metadata {
    size: u128,
    something_else: bool,
}

// If we want to add any additional metadata such as input data size, we must
// implement the Metadata trait, with a given number of columns.
impl energy_bench::Metadata<2> for Metadata {
    fn get_header() -> [&'static str; 2] {
        ["Size", "Something else"]
    }

    fn get_values(&self) -> [String; 2] {
        [self.size.to_string(), self.something_else.to_string()]
    }
}

fn main() {
    energy_bench::enable_logging(energy_bench::LogLevel::Info);

    let mut bench = EnergyBench::default("metadata");

    for size in [100_000, 1000_000, 10_000_000] {
        let metadata = Metadata {
            size,
            something_else: true,
        };

        bench.benchmark(metadata, &|| factorial(size));
    }
}
