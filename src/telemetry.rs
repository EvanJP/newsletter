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

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
// HRTB. Check https://doc.rust-lang.org/nomicon/hrtb.html.
// Essentially means `Sink` implements `MakeWriter` trait for all choices of
// lifetime parameter `'a`.
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
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

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber.into()).expect("Failed to set subscriber.");
}
