use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use chrono::Local;

struct FileLogger {
    file: Mutex<File>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} [{:>5}] {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        );
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

/// Appends to the log file and installs it as the global logger.
pub fn init(path: &Path) -> Result<(), String> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    log::set_boxed_logger(Box::new(FileLogger { file: Mutex::new(file) }))
        .map_err(|e| e.to_string())?;
    log::set_max_level(log::LevelFilter::Info);
    Ok(())
}
