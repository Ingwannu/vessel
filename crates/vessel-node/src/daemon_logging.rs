//! Binary logging policy; inherited backend environment diagnostics stay private.
use tracing_subscriber::{filter::LevelFilter, filter::Targets, prelude::*};

pub(super) fn init(attaching: bool) {
    init_with_writer(
        attaching,
        std::io::stderr,
        std::io::IsTerminal::is_terminal(&std::io::stderr()),
    );
}

pub(super) fn init_with_writer<W>(attaching: bool, writer: W, ansi: bool)
where
    W: for<'writer> tracing_subscriber::fmt::MakeWriter<'writer> + Send + Sync + 'static,
{
    let user = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_unset| if attaching { "warn" } else { "info" }.into());
    tracing_subscriber::fmt()
        .with_env_filter(user)
        .with_writer(writer)
        .with_ansi(ansi)
        .finish()
        // This independent directive is an upper bound, after the user filter.
        // Appending to EnvFilter alone would let a more specific user target win.
        // LogTracer checks each log record's original target before forwarding it.
        .with(
            Targets::new()
                .with_default(LevelFilter::TRACE)
                .with_target("portable_pty", LevelFilter::WARN),
        )
        .init();
}

#[cfg(test)]
mod tests;
