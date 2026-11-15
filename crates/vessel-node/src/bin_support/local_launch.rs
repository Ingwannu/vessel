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
    /// Dedicated readiness is emitted only into an explicitly managed child's pipe.
    #[arg(long, hide = true, action = clap::ArgAction::SetTrue, value_parser = desktop_parser(), conflicts_with = "detached")]
    desktop_managed: Desktop,
}

impl Mode {
    pub(super) fn desktop_managed(&self) -> bool {
        self.desktop_managed == Desktop::Enabled
    }
    pub(super) fn survives_hangup(&self, detached: bool) -> bool {
        detached || self.desktop_managed == Desktop::Enabled
    }

    pub(super) fn announce(&self, detached: bool, node: NodeId) -> std::io::Result<()> {
        let line = if self.desktop_managed == Desktop::Enabled {
            Some(vessel_proto::local_launch::Ready::new(node).encode())
        } else if detached {
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

#[cfg(test)]
mod tests {
    use crate::daemon_args::Args;
    use clap::Parser as _;

    #[test]
    fn managed_readiness_keeps_the_existing_attach_flag_and_refuses_ambiguous_modes() {
        let plain = Args::try_parse_from(["vesseld"]).unwrap();
        assert!(!plain.run.launch.desktop_managed());
        assert!(!plain.run.launch.survives_hangup(plain.run.detached));
        let managed = Args::try_parse_from(["vesseld", "--desktop-managed"]).unwrap();
        assert!(managed.run.launch.desktop_managed());
        assert!(managed.run.launch.survives_hangup(managed.run.detached));
        assert!(!managed.run.detached);
        let legacy = Args::try_parse_from(["vesseld", "--detached"]).unwrap();
        assert!(legacy.run.detached);
        assert!(!legacy.run.launch.desktop_managed());
        assert!(legacy.run.launch.survives_hangup(legacy.run.detached));
        assert!(Args::try_parse_from(["vesseld", "--desktop-managed", "--detached"]).is_err());
    }
}
