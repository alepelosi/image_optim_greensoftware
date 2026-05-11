use std::env;

use indexmap::{indexmap, IndexMap};
use serde::Deserialize;

pub struct Ina(f32);

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct InaResponse {
    power_draw: f32,
    steady_time: usize,
    measurements: usize,
    electricity_consumed_current: f32,
    electricity_consumed_total: f32,
}

impl Ina {
    pub fn now() -> Option<Self> {
        read().map(|e| Self(e))
    }

    pub fn elapsed(&self) -> IndexMap<String, f32> {
        let prev = self.0;
        let next = read().unwrap();
        indexmap! {
            String::from("INA system energy (J)") => next - prev,
        }
    }

    pub fn reset(&mut self) {
        self.0 = read().unwrap()
    }
}

fn read() -> Option<f32> {
    let url = env::var("ENERGY_STATS").ok()?;
    let ina = reqwest::blocking::get(url).and_then(|x| x.json::<InaResponse>());
    match ina {
        Ok(ina) => Some(ina.electricity_consumed_total),
        Err(e) => {
            log::error!("ENERGY_STATS is defined, but an error occurred: {e}");
            None
        }
    }
}
