mod hwmon;
mod ina;
mod nvml;

use indexmap::IndexMap;
use rapl_energy::Rapl;

use crate::{
    energy_acc::EnergyAccumulator,
    software_energy_lab::{hwmon::Hwmon, ina::Ina, nvml::Nvml},
};

pub struct SoftwareEnergyLab<'a> {
    rapl: Option<Rapl>,
    nvml: Option<Nvml<'a>>,
    ina: Option<Ina>,
    hwmon: Vec<Hwmon>,
}

impl SoftwareEnergyLab<'_> {
    pub fn new() -> Self {
        let rapl = Rapl::now(true);
        if rapl.as_ref().is_none_or(|r| r.packages.is_empty()) {
            log::warn!("No RAPL packages were found. Please ensure you have read access to the files in `/sys/class/powercap/intel-rapl`.");
        }
        Self {
            rapl,
            nvml: Nvml::now(),
            ina: Ina::now(),
            hwmon: Hwmon::get_available(),
        }
    }
}

impl EnergyAccumulator for SoftwareEnergyLab<'_> {
    fn elapsed(&self) -> indexmap::IndexMap<String, f32> {
        let mut res = IndexMap::new();

        if let Some(rapl) = &self.rapl {
            res.extend(rapl.elapsed());
        }

        if let Some(nvml) = &self.nvml {
            res.extend(nvml.elapsed());
        }

        if let Some(ina) = &self.ina {
            res.extend(ina.elapsed());
        }

        for hwmon in &self.hwmon {
            res.extend(hwmon.elapsed());
        }

        res
    }

    fn reset(&mut self) {
        for hwmon in &mut self.hwmon {
            hwmon.reset();
        }

        if let Some(ina) = &mut self.ina {
            ina.reset();
        }

        if let Some(nvml) = &mut self.nvml {
            nvml.reset();
        }

        if let Some(rapl) = &mut self.rapl {
            rapl.reset();
        }
    }
}
