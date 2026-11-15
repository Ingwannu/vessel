//! Agent command edges use existing domain types instead of generic option maps.

use super::WaitBudget;
use crate::selector::SessionSelector;
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;
use vessel_proto::{HarnessRef, OptionValueId, SessionTitle};

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Ls(AgentList),
    Run(Box<AgentRun>),
    Import {
        #[arg(value_parser = super::identifier::conversation)]
        id: vessel_proto::HarnessSessionId,
        #[arg(long)]
        provider: HarnessRef,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        label: Vec<String>,
    },
    Attach {
        id: SessionSelector,
    },
    Stop(Selection),
    Delete(Selection),
    Send(AgentSend),
    Inspect {
        id: SessionSelector,
    },
    Wait(Wait),
    Mode {
        id: SessionSelector,
        #[arg(value_parser = super::identifier::option)]
        mode: Option<OptionValueId>,
        #[arg(long, conflicts_with = "mode")]
        list: bool,
    },
    Archive {
        id: SessionSelector,
        #[arg(long)]
        force: bool,
    },
    Detach {
        id: SessionSelector,
    },
    Open {
        id: SessionSelector,
        #[arg(long)]
        server: Option<vessel_proto::NodeId>,
    },
    Update {
        id: SessionSelector,
        #[arg(long)]
        name: Option<SessionTitle>,
        #[arg(long, value_parser = super::identifier::option)]
        thinking: Option<OptionValueId>,
    },
}

#[derive(Debug, Default, Args)]
pub(crate) struct AgentList {
    #[arg(short = 'g', long)]
    pub(crate) global: bool,
    #[arg(short = 'a', long)]
    pub(crate) all: bool,
    #[arg(long)]
    pub(crate) background: bool,
    #[arg(long, value_parser = super::identifier::option)]
    pub(crate) thinking: Option<OptionValueId>,
    #[arg(long)]
    pub(crate) label: Vec<String>,
}

#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub(crate) struct Selection {
    pub(crate) id: Option<SessionSelector>,
    #[arg(long)]
    pub(crate) cwd: Option<PathBuf>,
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(Debug, Args)]
pub(crate) struct Wait {
    pub(crate) id: SessionSelector,
    #[arg(long, default_value_t = WaitBudget::DEFAULT)]
    pub(crate) timeout: WaitBudget,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Behavior {
    Steer,
    Interrupt,
}
impl From<Behavior> for vessel_proto::Delivery {
    fn from(value: Behavior) -> Self {
        match value {
            Behavior::Steer => Self::Steer,
            Behavior::Interrupt => Self::Interrupt,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct AgentSend {
    /// Positional order matches Paseo: prompt first, then agent ID.
    #[arg(required_unless_present_any = ["prompt", "prompt_file"])]
    pub(crate) text: Option<String>,
    pub(crate) id: SessionSelector,
    #[arg(long, conflicts_with_all = ["text", "prompt_file"])]
    pub(crate) prompt: Option<String>,
    #[arg(long, conflicts_with = "text")]
    pub(crate) prompt_file: Option<PathBuf>,
    #[arg(long)]
    pub(crate) image: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) behavior: Option<Behavior>,
    #[arg(long)]
    pub(crate) no_wait: bool,
}

#[derive(Debug, Args)]
pub(crate) struct AgentRun {
    pub(crate) prompt: String,
    #[arg(long)]
    pub(crate) provider: HarnessRef,
    #[arg(long, value_parser = super::identifier::model)]
    pub(crate) model: Option<vessel_proto::ModelId>,
    #[arg(long)]
    pub(crate) cwd: Option<PathBuf>,
    #[arg(long, alias = "name")]
    pub(crate) title: Option<SessionTitle>,
    #[arg(long, value_parser = super::identifier::option)]
    pub(crate) mode: Option<OptionValueId>,
    #[arg(long, value_parser = super::identifier::option)]
    pub(crate) thinking: Option<OptionValueId>,
    #[arg(long)]
    pub(crate) label: Vec<String>,
    #[arg(long)]
    pub(crate) env: Vec<String>,
    #[arg(long)]
    pub(crate) image: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) workspace: Option<String>,
    #[arg(long)]
    pub(crate) output_schema: Option<String>,
    #[arg(long, default_value_t = WaitBudget::DEFAULT)]
    pub(crate) wait_timeout: WaitBudget,
    #[arg(long, alias = "detach")]
    pub(crate) no_wait: bool,
}
