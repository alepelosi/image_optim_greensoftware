use std::{
    ops::Index,
    path::PathBuf,
    thread,
    time::Duration,
};

use file_backed_value::FileBackedValue;
use gethostname::gethostname;
use indexmap::IndexMap;

use crate::EnergyAccumulator;

pub struct IdlePower(IndexMap<String, f32>);

impl IdlePower {
    pub fn init(path: Option<&PathBuf>, idle_seconds: usize, probes: &mut Box<dyn EnergyAccumulator>) -> Self {
        let filename = gethostname()
            .to_ascii_lowercase()
            .to_str()
            .map_or(format!("idle-{idle_seconds}s.json"), |user| {
                format!("idle-{idle_seconds}s-{user}.json")
            });

        let mut file = if let Some(path) = path {
            FileBackedValue::new_at(&filename, &path)
        } else {
            FileBackedValue::new(&filename)
        };

        log::info!("Idle file (will be) stored at {:?}", file.path());
        file.set_dirty_time(Duration::from_hours(18));

        let idle = match file.get_or_insert_with(|| measure_idle(probes, idle_seconds)) {
            Ok(idle) => idle.clone(),
            Err(e) => {
                log::error!("Error reading idle file: {:?}", e);
                log::warn!("Recalculating idle due to file error");
                measure_idle(probes, idle_seconds)
            },
        };

        Self(idle)
    }
}

impl Index<&str> for IdlePower {
    type Output = f32;

    fn index(&self, key: &str) -> &Self::Output {
        &self.0[key]
    }
}

fn measure_idle(probes: &mut Box<dyn EnergyAccumulator>, idle_seconds: usize) -> IndexMap<String, f32> {
    if idle_seconds > 0 {
        let before = idle_seconds.min(30);
        log::info!("Waiting for system to stabilise for {}s before measuring idle", before);
        thread::sleep(Duration::from_secs(before as u64));
    }

    log::info!("Measuring idle for {}s", idle_seconds);

    probes.reset();
    thread::sleep(Duration::from_secs(1));
    let mut min_power = probes.elapsed();

    for _ in 1..idle_seconds {
        probes.reset();
        thread::sleep(Duration::from_secs(1));
        for (k, v) in probes.elapsed() {
            min_power[&k] = min_power[&k].min(v);
        }
    }

    log::info!("Idle power draw: {:#?}", min_power);
    min_power
}
