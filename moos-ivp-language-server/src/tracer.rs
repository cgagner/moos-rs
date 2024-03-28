use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter, Registry};

pub struct Tracer {}

impl Tracer {
    pub fn init() -> anyhow::Result<()> {
        // TODO: This should eventually take the filter as part of the init.
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env()?
            .add_directive("moos_ivp_language_server=debug".parse()?)
            .add_directive("moos_parser=trace".parse()?);
        // TODO: This should eventually allow logging to a file if an
        // environment variable is set.
        let writer = BoxMakeWriter::new(std::io::stderr);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_filter(filter);

        Registry::default().with(fmt_layer).try_init()?;

        Ok(())
    }
}
