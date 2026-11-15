//! Source spelling admission before installation effects. This stage has no
//! network, stat, executable resolution, credential or source-file authority.
mod npm;
mod reference;
mod registry;
#[cfg(test)]
mod tests;

use npm::{NpmSource, npm};
use reference::{Prefix, Shape, SourceText, reference};
use registry::{RegistryEndpoint, RegistryId, RegistrySource};
use std::fmt;
use vessel_proto::workspace::RelPath;

#[derive(Debug, thiserror::Error)]
enum SourceError {
    #[error("plugin source must be bounded, nonempty and free of edge whitespace/controls")]
    Text,
    #[error("invalid npm plugin package or selector")]
    Npm,
    #[error("invalid native plugin registry origin or slug")]
    Registry,
    #[error("a compatible native plugin registry must be configured")]
    NoRegistry,
    #[error("registry sources own their reviewed subpath and revision")]
    RegistryOverride,
    #[error("invalid source URL")]
    Url(#[source] url::ParseError),
}

/// A directory spelling is NOT a validated filesystem path/capability. The
/// caller must resolve/validate it in the actual node/client context first.
#[derive(Clone)]
struct DirectoryInput(SourceText);
impl fmt::Debug for DirectoryInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DirectoryInput(<unresolved private path>)")
    }
}

/// Full Git acquisition policy is deliberately not inferred from a spelling.
/// Until its real URL/argv/pin adapter exists, this is unresolved input only.
#[derive(Debug, Clone)]
struct GitInput {
    prefix: Prefix,
    source: SourceText,
}

#[derive(Debug)]
enum Source {
    Directory {
        input: DirectoryInput,
        subpath: Option<RelPath>,
    },
    Npm {
        input: NpmSource,
        subpath: Option<RelPath>,
    },
    Registry(RegistrySource),
    Git {
        input: GitInput,
        subpath: Option<RelPath>,
    },
}

fn parse(raw: &str, configured_registry: Option<&RegistryEndpoint>) -> Result<Source, SourceError> {
    let parsed = reference(raw)?;
    if parsed.shape == Shape::Directory {
        return Ok(Source::Directory {
            input: DirectoryInput(parsed.source),
            subpath: parsed.subpath,
        });
    }
    // Host:port registry references must be recognized on the original input,
    // before a colon could be mistaken for a package subpath.
    if parsed.prefix == Prefix::None {
        if let Some(registry) = registry::explicit(raw)? {
            return Ok(Source::Registry(registry));
        }
        if let Ok(id) = RegistryId::parse(parsed.source.as_str()) {
            if parsed.subpath.is_some() {
                return Err(SourceError::RegistryOverride);
            }
            let endpoint = configured_registry.ok_or(SourceError::NoRegistry)?.clone();
            return Ok(Source::Registry(RegistrySource { endpoint, id }));
        }
    }
    if parsed.prefix == Prefix::Npm {
        return Ok(Source::Npm {
            input: npm(parsed.source.as_str())?,
            subpath: parsed.subpath,
        });
    }
    if parsed.prefix == Prefix::None
        && let Ok(input) = npm(parsed.source.as_str())
    {
        return Ok(Source::Npm {
            input,
            subpath: parsed.subpath,
        });
    }
    Ok(Source::Git {
        input: GitInput {
            prefix: parsed.prefix,
            source: parsed.source,
        },
        subpath: parsed.subpath,
    })
}
