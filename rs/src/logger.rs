use std::fs::File;
use std::io::{LineWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

struct FileLogger {
    file: Mutex<LineWriter<File>>,
    debug_to_console: bool,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
            let level = match record.level() {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN ",
                log::Level::Info => "INFO ",
                log::Level::Debug => "DEBUG",
                log::Level::Trace => "TRACE",
            };
            let now = chrono::Local::now();
            let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");
            let target = record.target();
            let message = format!(
                "[{}] [{}] [{}] {}",
                timestamp,
                level,
                target,
                record.args()
            );
            let _ = writeln!(file, "{}", message);
            let _ = file.flush();

            if self.debug_to_console {
                println!("{}", message);
            }
        }
    }

    fn flush(&self) {
        let _ = self.file.lock().unwrap_or_else(|e| e.into_inner()).flush();
    }
}

impl Drop for FileLogger {
    fn drop(&mut self) {
        self.flush();
    }
}

pub fn init(debug_to_console: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    if debug_to_console {
        unsafe {
            use windows::Win32::System::Console::AllocConsole;
            AllocConsole();
        }
    }

    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("gorun_debug.log")))
        .unwrap_or_else(|| PathBuf::from("gorun_debug.log"));

    let file = File::create(&log_path)?;
    let logger = FileLogger {
        file: Mutex::new(LineWriter::new(file)),
        debug_to_console,
    };

    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Debug);

    log::debug!("Logger initialized at: {:?}", log_path);

    Ok(())
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unknown panic"
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".to_string());
        log::error!("PANIC at {}: {}", location, msg);
        log::logger().flush();
    }));
}
