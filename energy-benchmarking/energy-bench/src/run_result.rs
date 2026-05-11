use std::time::Duration;

use indexmap::IndexMap;

use crate::IdlePower;

#[derive(Debug)]
pub struct RunResult {
    pub repeats: usize,
    pub runtime: Duration,
    pub energy: IndexMap<String, f32>,
}

impl RunResult {
    pub fn new(repeats: usize, runtime: Duration, energy: IndexMap<String, f32>) -> Self {
        Self {
            repeats,
            runtime,
            energy,
        }
    }

    /// The overhead measurement already includes its own idle energy,
    /// so we must subtract overhead before we subtract remaining idle
    /// to avoid subtracting idle twice.
    pub fn subtract_overhead(&mut self, overhead: RunResult) {
        debug_assert_eq!(overhead.repeats, 0);
        self.runtime = self.runtime.saturating_sub(overhead.runtime);
        self.energy.iter_mut().for_each(|(key, energy)| {
            *energy -= overhead.energy[key];
            *energy = energy.max(0f32);
        });
    }

    pub fn subtract_idle(&mut self, idle: &IdlePower) {
        self.energy.iter_mut().for_each(|(key, energy)| {
            *energy -= idle[key] * self.runtime.as_secs_f32();
            *energy = energy.max(0f32);
        });
    }

    pub fn normalise(&mut self) {
        if self.repeats > 1 {
            self.runtime /= self.repeats as u32;
            self.energy.values_mut().for_each(|energy| {
                *energy /= self.repeats as f32;
                *energy = energy.max(0f32);
            });
            self.repeats = 1;
        }
    }
}
