//! Read-only access to the original game installation, install discovery, and log capture.

mod install;
mod log;
pub mod steam;

pub use install::{InstallNotFound, InstallRoot, find_install};
pub use log::{LogLevel, LogSink, MemoryLog, StderrLog};
