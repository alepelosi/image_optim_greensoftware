use std::{ffi, mem};

use energy_bench::{EnergyAccumulator, IdlePower, software_energy_lab::SoftwareEnergyLab};
use indexmap::IndexMap;

struct BenchStart {
    probes: Box<dyn EnergyAccumulator>,
    idle: Option<IdlePower>,
}

#[repr(C)]
pub struct BenchResult {
    keys: *const *mut ffi::c_char,
    values: *const f32,
    len: usize,
}

impl BenchResult {
    pub fn from(measurements: IndexMap<String, f32>) -> Self {
        let len = measurements.len();
        let (keys, mut values): (Vec<String>, Vec<f32>) = measurements.into_iter().unzip();

        let mut cchar_vec: Vec<*mut ffi::c_char> = keys
            .into_iter()
            .map(|s| ffi::CString::new(s).unwrap().into_raw())
            .collect();

        cchar_vec.shrink_to_fit();
        values.shrink_to_fit();

        let res = BenchResult {
            keys: cchar_vec.as_ptr(),
            values: values.as_ptr(),
            len,
        };

        mem::forget(cchar_vec);
        mem::forget(values);
        res
    }

    pub fn free(&self) {
        let keys =
            unsafe { Vec::from_raw_parts(self.keys as *mut *mut ffi::c_char, self.len, self.len) };
        for key in keys {
            let cstr = unsafe { ffi::CString::from_raw(key) };
            drop(cstr);
        }

        let values = unsafe { Vec::from_raw_parts(self.values as *mut f32, self.len, self.len) };
        drop(values);
    }
}

#[unsafe(no_mangle)]
extern "C" fn energy_bench_init(idle_duration_seconds: usize) -> *mut BenchStart {
    let mut probes: Box<dyn EnergyAccumulator> = Box::new(SoftwareEnergyLab::new());

    let idle = if idle_duration_seconds > 0 {
        Some(IdlePower::init(None, idle_duration_seconds, &mut probes))
    } else {
        None
    };

    probes.reset();
    Box::into_raw(Box::new(BenchStart { probes, idle }))
}

#[unsafe(no_mangle)]
extern "C" fn energy_bench_start(start: &mut BenchStart) {
    start.probes.reset();
}

#[unsafe(no_mangle)]
extern "C" fn energy_bench_stop(start: &mut BenchStart) -> *mut BenchResult {
    let mut measurements = start.probes.elapsed();

    // Subtract idle
    if let Some(idle) = &start.idle {
        measurements
            .iter_mut()
            .filter(|(key, _)| *key != "runtime")
            .for_each(|(key, value)| {
                *value -= idle[key] * idle["runtime"];
            });
    }

    Box::into_raw(Box::new(BenchResult::from(measurements)))
}

#[unsafe(no_mangle)]
extern "C" fn energy_bench_free(res: &mut BenchStart) {
    let res = unsafe { Box::from_raw(res) };
    drop(res);
}

#[unsafe(no_mangle)]
extern "C" fn energy_bench_res_free(res: &mut BenchResult) {
    let res = unsafe { Box::from_raw(res) };
    res.free();
    drop(res);
}
