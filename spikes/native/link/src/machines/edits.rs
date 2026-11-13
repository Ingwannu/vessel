//! File boundary: preserve typed sources, classify without printing their text.
use super::Machine;
use std::path::Path;
use vessel_app_view::machine_book::{Edit, Failure, Name};
use vessel_client::book::{BookError, BookFile, NodeName};

/// An error kept at the file boundary, never formatted for a screen or a log.
#[derive(Debug, thiserror::Error)]
pub(super) enum BookFailure {
    #[error("the machine book could not be read")]
    Read(#[source] BookError),
    #[error("the machine book could not be changed")]
    Write(#[source] BookError),
    #[error("a validated name was not accepted by the client")]
    Name(#[source] vessel_client::book::InvalidNodeName),
    #[error("the book returned an invalid machine name")]
    BookName,
}

impl BookFailure {
    pub(super) const fn failure(&self) -> Failure {
        let error = match self {
            Self::Read(error) | Self::Write(error) => error,
            Self::Name(_) | Self::BookName => return Failure::InvalidBook,
        };
        match error {
            BookError::Io { .. } => match self {
                Self::Read(_) => Failure::Read,
                Self::Write(_) => Failure::Write,
                Self::Name(_) | Self::BookName => Failure::InvalidBook,
            },
            BookError::Exists { .. } => Failure::Taken,
            BookError::Missing { .. } => Failure::Missing,
            BookError::Syntax { .. }
            | BookError::ProjectSyntax { .. }
            | BookError::Entry { .. }
            | BookError::UnknownDefault { .. }
            | BookError::Unknown { .. }
            | BookError::PinnedElsewhere { .. } => Failure::InvalidBook,
        }
    }
}

pub(super) fn read(
    path: &Path,
    node_of: impl Fn(&str) -> Option<(vessel_proto::NodeInfo, vessel_proto::ProtocolVersion)>,
) -> Result<Vec<Machine>, BookFailure> {
    let file = BookFile::open(path).map_err(BookFailure::Read)?;
    let book = file.book();
    book.nodes
        .iter()
        .map(|(name, entry)| {
            let (node, protocol) = node_of(name.as_str()).unzip();
            Ok(Machine {
                name: Name::parse(name.as_str()).map_err(|_invalid| BookFailure::BookName)?,
                transport: match &entry.address {
                    vessel_client::NodeAddress::Ws(_) => {
                        vessel_app_view::machines::Transport::Direct
                    }
                    vessel_client::NodeAddress::Ssh(_) => vessel_app_view::machines::Transport::Ssh,
                    vessel_client::NodeAddress::Secure(_) => {
                        vessel_app_view::machines::Transport::Paired
                    }
                },
                default: book.default_node.as_ref() == Some(name),
                node,
                protocol,
            })
        })
        .collect()
}

pub(super) fn write(path: &Path, edit: &Edit) -> Result<(), BookFailure> {
    let mut file = BookFile::open(path).map_err(BookFailure::Read)?;
    match edit {
        Edit::Rename { from, to } => {
            let to = to.as_str().parse::<NodeName>().map_err(BookFailure::Name)?;
            file.rename(from.as_str(), &to)
        }
        Edit::MakeDefault(name) => file.set_default(Some(name.as_str())),
        Edit::Forget(name) => file.remove(name.as_str()),
    }
    .map_err(BookFailure::Write)
}

#[cfg(test)]
mod tests;
