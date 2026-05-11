use std::{fs::{self, File}, marker::PhantomData, path::Path};

use crate::{Metadata, run_result::RunResult};

pub struct OutputFiles<MD: Metadata<COLS>, const COLS: usize> {
    name: String,
    date: String,
    results_wtr: Option<csv::Writer<File>>,
    summary_wtr: Option<csv::Writer<File>>,
    _phantom: PhantomData<MD>,
}

impl<MD: Metadata<COLS>, const COLS: usize> OutputFiles<MD, COLS> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            date: chrono::offset::Local::now()
                .format("%Y-%m-%d-%H-%M-%S")
                .to_string(),
            results_wtr: None,
            summary_wtr: None,
            _phantom: PhantomData,
        }
    }

    pub fn write_results(
        &mut self,
        metadata: &MD,
        run_results: &Vec<RunResult>,
    ) -> csv::Result<()> {
        let wtr = self.results_wtr.get_or_insert_with(|| {
            let filename = format!("{}-{}.csv", self.name, self.date);
            Self::init_columns(&filename, false, run_results[0].energy.keys()).unwrap()
        });

        for run in run_results {
            for val in metadata.get_values() {
                wtr.write_field(val)?;
            }

            wtr.write_field(run.runtime.as_secs_f32().to_string())?;
            for val in run.energy.values() {
                wtr.write_field(val.to_string())?;
            }

            wtr.write_record(None::<&[u8]>)?;
        }

        Ok(())
    }

    pub fn write_summary(
        &mut self,
        metadata: &MD,
        run_results: &Vec<RunResult>,
    ) -> csv::Result<()> {
        let wtr = self.summary_wtr.get_or_insert_with(|| {
            let filename = format!("{}-avg-{}.csv", self.name, self.date);
            Self::init_columns(&filename, true, run_results[0].energy.keys()).unwrap()
        });

        for val in metadata.get_values() {
            wtr.write_field(val)?;
        }

        let runtimes: Vec<f32> = run_results
            .iter()
            .map(|r| r.runtime.as_secs_f32())
            .collect();
        let mu = statistical::mean(&runtimes);
        let sd = statistical::population_standard_deviation(&runtimes, Some(mu));
        wtr.write_field(mu.to_string())?;
        wtr.write_field(sd.to_string())?;

        for k in run_results[0].energy.keys() {
            let energies: Vec<f32> = run_results.iter().map(|r| r.energy[k]).collect();
            let mu = statistical::mean(&energies);
            let sd = statistical::population_standard_deviation(&energies, Some(mu));
            wtr.write_field(mu.to_string())?;
            wtr.write_field(sd.to_string())?;
        }

        wtr.write_record(None::<&[u8]>)
    }

    fn init_columns<'a>(
        filename: &str,
        include_sd: bool,
        keys: impl Iterator<Item = &'a String>,
    ) -> csv::Result<csv::Writer<File>> {
        let mut wtr = create_file(filename)?;

        for key in MD::get_header() {
            wtr.write_field(key)?;
        }

        wtr.write_field("Runtime")?;
        if include_sd {
            wtr.write_field("Runtime SD")?;
        }

        for key in keys {
            wtr.write_field(key)?;
            if include_sd {
                wtr.write_field(format!("{} SD", key))?;
            }
        }

        wtr.write_record(None::<&[u8]>)?;
        Ok(wtr)
    }
}

fn create_file(filename: &str) -> csv::Result<csv::Writer<File>> {
    const ROOT_DIR: &str = "energy-bench";
    let filename = sanitize_filename::sanitize(filename);
    let path = Path::new(ROOT_DIR).join(filename);
    fs::create_dir_all(ROOT_DIR)?;
    csv::Writer::from_path(path)
}
