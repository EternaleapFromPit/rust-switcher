use std::sync::OnceLock;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

static TRACING_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init_tracing() {
    let default_filter = if cfg!(debug_assertions) {
        "trace"
    } else {
        "info"
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    if cfg!(debug_assertions) {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .with_level(true)
            .with_target(true);

        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer);

        let _ = subscriber.try_init();
        return;
    }

    let file_appender = tracing_appender::rolling::hourly("./logs", "output.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_level(true)
        .with_target(true);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    if subscriber.try_init().is_ok() {
        let _ = TRACING_GUARD.set(guard);
    }
}
