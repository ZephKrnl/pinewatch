use std::fmt;

/// The game's log levels (`UnturnedLog.info`, `warn`, `error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        })
    }
}

/// Where ported code sends the messages the original logs. The text is exactly what the original logs.
pub trait LogSink {
    fn log(&mut self, level: LogLevel, text: &str);
}

/// Keeps every message in order. Used by tests and tools.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryLog {
    pub entries: Vec<(LogLevel, String)>,
}

impl LogSink for MemoryLog {
    fn log(&mut self, level: LogLevel, text: &str) {
        self.entries.push((level, text.to_string()));
    }
}

/// Prints each message to standard error, prefixed with its level.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrLog;

impl LogSink for StderrLog {
    fn log(&mut self, level: LogLevel, text: &str) {
        eprintln!("[{level}] {text}");
    }
}

/// `UnturnedLog.exception(e, message)`: `message` as an error, then the exception's message
/// (`(empty exception message)` if it has none), then the same for each inner exception. The original logs a stack
/// trace after each exception message; Pinewatch has none to log (`COMPATIBILITY.md`, L1).
pub fn log_exception(log: &mut dyn LogSink, exception: &pinewatch_mono::MonoException, message: &str) {
    log.log(LogLevel::Error, message);
    log_exception_only(log, exception);
}

/// `UnturnedLog.exception(e)`: the exception's message and its inner exceptions' messages, as in [`log_exception`].
pub fn log_exception_only(log: &mut dyn LogSink, exception: &pinewatch_mono::MonoException) {
    let mut current = Some(exception);
    while let Some(exception) = current {
        let text = if exception.message.is_empty() { "(empty exception message)" } else { exception.message.as_str() };
        log.log(LogLevel::Error, text);
        current = exception.inner.as_deref();
    }
}
