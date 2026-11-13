use std::fs::File;
use std::io::{LineWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

/// 日志轮转阈值：超过后轮转为 .old（保留一份，防止无界增长）。
const ROTATE_THRESHOLD: usize = 10 * 1024 * 1024;

/// 控制字符清洗：防止路径/文件名含换行/ESC 等伪造日志条目或终端转义（日志注入防护）。
fn sanitize(msg: &str) -> String {
    msg.chars()
        .map(|c| match c {
            '\r' => "\\r".to_string(),
            '\n' => "\\n".to_string(),
            '\t' => "\\t".to_string(),
            c if c.is_control() => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

struct Inner {
    file: Option<LineWriter<File>>,
    bytes: usize,
}

struct FileLogger {
    file: Mutex<Inner>,
    path: PathBuf,
    debug_to_console: bool,
}

impl FileLogger {
    fn rotate(&self, inner: &mut Inner) {
        // 注意：本方法在持有 self.file 锁的调用方内执行，禁止再调用 log::*
        // （否则非重入 Mutex 死锁）。失败仅降级为静默，避免后续日志丢失。
        drop(inner.file.take());

        let old = self.path.with_extension("log.old");
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&self.path, &old);

        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            inner.file = Some(LineWriter::new(file));
        }
        inner.bytes = 0;
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut inner = self.file.lock().unwrap_or_else(|e| e.into_inner());
            let level = match record.level() {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN ",
                log::Level::Info => "INFO ",
                log::Level::Debug => "DEBUG",
                log::Level::Trace => "TRACE",
            };
            let now = chrono::Local::now();
            let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f");
            let target = sanitize(record.target());
            let message = sanitize(&record.args().to_string());
            let line = format!("[{}] [{}] [{}] {}", timestamp, level, target, message);

            if let Some(writer) = inner.file.as_mut() {
                let _ = writeln!(writer, "{}", line);
                let _ = writer.flush();
                inner.bytes += line.len() + 1;
                if inner.bytes >= ROTATE_THRESHOLD {
                    self.rotate(&mut inner);
                }
            }

            if self.debug_to_console {
                println!("{}", line);
            }
        }
    }

    fn flush(&self) {
        let mut inner = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(writer) = inner.file.as_mut() {
            let _ = writer.flush();
        }
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
            // --debug 时分配控制台用于输出日志；失败不影响主流程（继续写文件日志）
            let _ = AllocConsole();
        }
    }

    let log_path = crate::config::config_path()
        .parent()
        .map(|d| d.join("gorun_debug.log"))
        .unwrap_or_else(|| PathBuf::from("gorun_debug.log"));
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // 启动时轮转：旧日志超阈值则先备份为 .old
    if let Ok(meta) = std::fs::metadata(&log_path) {
        if meta.len() as usize >= ROTATE_THRESHOLD {
            let old = log_path.with_extension("log.old");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(&log_path, &old);
        }
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let logger = FileLogger {
        file: Mutex::new(Inner {
            file: Some(LineWriter::new(file)),
            bytes: 0,
        }),
        path: log_path.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_sanitize_newlines() {
        assert_eq!(sanitize("a\nb"), "a\\nb");
        assert_eq!(sanitize("a\rb"), "a\\rb");
        assert_eq!(sanitize("a\tb"), "a\\tb");
        assert_eq!(sanitize("a\x1bb"), "a\\u001bb");
        assert_eq!(sanitize("normal"), "normal");
    }

    #[test]
    fn test_rotate_threshold_constant() {
        assert!(ROTATE_THRESHOLD >= 10 * 1024 * 1024);
    }

    #[test]
    fn test_log_path_under_base() {
        let p = Path::new("x/gorun_debug.log");
        assert_eq!(p.with_extension("log.old").to_string_lossy(), "x/gorun_debug.log.old");
    }
}
