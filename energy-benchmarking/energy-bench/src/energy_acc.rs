use indexmap::indexmap;
use rapl_energy::Rapl;

pub trait EnergyAccumulator {
    /// Gets the difference in value since the last time this energy probe was created/reset.
    fn elapsed(&self) -> indexmap::IndexMap<String, f32>;

    /// Resets this probe, such that the next time `elapsed` is called,
    /// the difference compared to the value at this reset is returned.
    fn reset(&mut self);
}

pub struct DefaultEnergyAccumulator {
    rapl: Option<Rapl>,
}

impl DefaultEnergyAccumulator {
    pub fn new() -> Self {
        let rapl = Rapl::now(true);
        if rapl.as_ref().is_none_or(|r| r.packages.is_empty()) {
            log::warn!("No RAPL packages were found. Please ensure you have read access to the files in `/sys/class/powercap/intel-rapl`.");
        }
        Self { rapl }
    }
}

impl EnergyAccumulator for DefaultEnergyAccumulator {
    fn elapsed(&self) -> indexmap::IndexMap<String, f32> {
        self.rapl.as_ref().map_or(indexmap! {}, |x| x.elapsed())
    }

    fn reset(&mut self) {
        if let Some(rapl) = &mut self.rapl {
            rapl.reset();
        }
    }
}
