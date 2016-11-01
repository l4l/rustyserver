extern crate log;

use log::{LogLevelFilter, LogRecord, LogLevel, LogMetadata, SetLoggerError};

struct SimpleLogger;

const LEVEL: (LogLevelFilter, LogLevel) = (LogLevelFilter::Trace, LogLevel::Trace);

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LEVEL.0);
        Box::new(SimpleLogger)
    })
}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LEVEL.1
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}
