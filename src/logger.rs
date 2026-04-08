use tracing_appender::non_blocking::WorkerGuard;

use crate::config::Config;
use crate::error::{MempalaceError, Result};

pub fn init() -> Result<WorkerGuard> {
    let log_dir = Config::default_config_dir().join("logs");
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "mempalace.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .with_writer(non_blocking)
        .try_init()
        .map_err(|error| {
            MempalaceError::Config(format!("failed to initialize logging: {error}"))
        })?;

    Ok(guard)
}
