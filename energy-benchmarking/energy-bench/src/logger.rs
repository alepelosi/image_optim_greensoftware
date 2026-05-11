struct EnergyBenchLogger;

impl log::Log for EnergyBenchLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        // No check needed because we already do `set_max_level`
        // TODO: in the future, after the Green Software course concludes,
        // we should let users on the outside handle log levels through the env_logger crate.
        // Currently: `set_max_level` will change the entire application, not just energy-bench
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Wrapper around log::LevelFilter, we do this because:
/// - for use with `clap`
/// - users of `energy-bench` do not have to include the log library just to set the logging level.
#[repr(usize)]
#[derive(Clone, Copy)]
pub enum LogLevel {
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Into<log::LevelFilter> for LogLevel {
    fn into(self) -> log::LevelFilter {
        use LogLevel::*;
        match self {
            Off => log::LevelFilter::Off,
            Trace => log::LevelFilter::Trace,
            Debug => log::LevelFilter::Debug,
            Info => log::LevelFilter::Info,
            Warn => log::LevelFilter::Warn,
            Error => log::LevelFilter::Error,
        }
    }
}

pub fn enable_logging(log_level: LogLevel) {
    static LOGGER: EnergyBenchLogger = EnergyBenchLogger;
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log_level.into()))
        .unwrap();
}
