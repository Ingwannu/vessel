//! Binary-edge launch flags and private-pipe readiness; no local launcher dependency.

use clap::builder::TypedValueParser as _;
use std::io::Write as _;
use vessel_proto::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Desktop {
    #[default]
    Disabled,
    Enabled,
}

fn desktop_parser() -> impl clap::builder::TypedValueParser<Value = Desktop> {
    clap::builder::BoolValueParser::new().map(|enabled| {
        if enabled {
            Desktop::Enabled
        } else {
            Desktop::Disabled
        }
    })
}

#[derive(Debug, clap::Args)]
pub(super) struct Mode {
    /// Existing attach startup marker and hangup survival remain unchanged.
    #[arg(long, hide = true)]
    detached: bool,
    /// Dedicated readiness is emitted only into an explicitly managed child's pipe.
    #[arg(long, hide = true, action = clap::ArgAction::SetTrue, value_parser = desktop_parser(), conflicts_with = "detached")]
    desktop_managed: Desktop,
}

impl Mode {
    pub(super) fn desktop_managed(&self) -> bool {
        self.desktop_managed == Desktop::Enabled
    }
    pub(super) fn survives_hangup(&self) -> bool {
        self.detached || self.desktop_managed == Desktop::Enabled
    }

    pub(super) fn announce(&self, node: NodeId) -> std::io::Result<()> {
        let line = if self.desktop_managed == Desktop::Enabled {
            Some(vessel_proto::local_launch::Ready::new(node).encode())
        } else if self.detached {
            Some(format!(
                "{} {}\n",
                vessel_node::attach::READY,
                std::process::id()
            ))
        } else {
            None
        };
        if let Some(line) = line {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(line.as_bytes())?;
            stdout.flush()?;
        }
        Ok(())
    }
}
