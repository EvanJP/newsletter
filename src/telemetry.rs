//! The logging system for the app.
use tokio::task::JoinHandle;
use tracing::dispatcher::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::BunyanFormattingLayer;
// Processes and stores in JSON.
use tracing_bunyan_formatter::JsonStorageLayer;
// logs are not collected by tracing, this is necessary for it.
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
// Discards spans based on log levels and their origins.
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

/// Returns a [`tracing_subscriber::Registry`] subscriber to write logs to.
///
/// Pipes logs through as such:
///
/// * `tracing_subscriber::EnvFilter` - Discards spans based on log levels and
///   origins.
/// * `tracing_bunyan_formatter::JsonStorageLayer` - Processes spans data and
///   stores in JSON format. Propagates context from parent spans to children.
/// * `tracing_bunyan_formatter::BunyanFormatterLayer` - Outputs log records in
///   bunyan-compatible JSON format.
///
/// # Arguments
///
/// * `name` - A string that will be attached to all logs.
/// * `env_filter` - A string representing the log level. Defaults to
///   info-level.
/// * `sink` - The `Sink` to write logs to.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
// HRTB. Check https://doc.rust-lang.org/nomicon/hrtb.html.
// Essentially means `Sink` implements `MakeWriter` trait for all choices of
// lifetime parameter `'a`.
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static, {
    // Fall back to info-level or above if RUST_LOG is not set.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));
    // Layers are for processing pipeline.
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    // Registry collects and stores span data and their relationships.
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Initializes the given subscriber.
///
/// Initializes a [`LogTracer`] as well, that pipes `log`s to `tracing`.
///
/// # Arguments
///
/// * `subscriber` - The `tracing` Subscriber to use for logging.
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber.into()).expect("Failed to set subscriber.");
}

/// Run the function in a blocking thread with the calling span attached.
///
/// [`tracing::Span`] uses the *thread's* span to generate its tree. This
/// function uses the current thread's span in the new thread.
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static, {
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
