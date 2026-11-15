//! The command line, parsed into typed commands. Hand-written: four
//! commands with a few options each do not need an argument parser in the
//! release tool's tree.

use std::{ffi::OsString, path::PathBuf};

use vessel_app_updater::{ArtifactPath, Channel, Label, Packaging, Role, Target};

pub(crate) const USAGE: &str = "\
usage:
  vessel-app-feed keygen DIR
  vessel-app-feed publish --key FILE --feed DIR --channel NAME --label LABEL
                          [--notes FILE] [--published TEXT]
                          --artifact TARGET:PACKAGING:PATH ...
                          [--companion TARGET:PACKAGING:ROLE:PATH ...]
                          [--compressed gzip]
  vessel-app-feed sums --key FILE --label LABEL DIR
  vessel-app-feed check --feed DIR --public KEY --channel NAME
  vessel-app-feed check-sums --public KEY DIR";

/// One artifact named on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactArg {
    pub(crate) target: Target,
    pub(crate) packaging: Packaging,
    pub(crate) path: ArtifactPath,
    /// The programs shipped and replaced with it (ADR 0094), from the
    /// `--companion` options naming its target and packaging.
    pub(crate) companions: Vec<CompanionArg>,
}

/// One companion named on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanionArg {
    pub(crate) role: Role,
    pub(crate) path: ArtifactPath,
}

/// What `publish` was asked to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Publish {
    pub(crate) key: PathBuf,
    pub(crate) feed: PathBuf,
    pub(crate) channel: Channel,
    pub(crate) label: Label,
    pub(crate) notes: Option<PathBuf>,
    pub(crate) published: Option<String>,
    pub(crate) artifacts: Vec<ArtifactArg>,
    /// `--compressed gzip`: every artifact and companion also has
    /// `<path>.gz`, offered to the apps that read it.
    pub(crate) gzip: bool,
}

/// A command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Keygen {
        dir: PathBuf,
    },
    Publish(Publish),
    Sums {
        key: PathBuf,
        label: Label,
        dir: PathBuf,
    },
    Check {
        feed: PathBuf,
        public: String,
        channel: Channel,
    },
    CheckSums {
        public: String,
        dir: PathBuf,
    },
}

/// A command line that is not one of the commands.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub(crate) struct ArgsError(String);

fn bad(text: impl Into<String>) -> ArgsError {
    ArgsError(text.into())
}

pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, ArgsError> {
    let args: Vec<String> = args
        .into_iter()
        .map(|arg| {
            arg.into_string()
                .map_err(|_raw| bad("an argument is not UTF-8"))
        })
        .collect::<Result<_, _>>()?;
    let mut args = args.into_iter();
    let command = args.next().ok_or_else(|| bad("no command"))?;
    let mut options = Options::default();
    let mut positional = Vec::new();
    while let Some(arg) = args.next() {
        if let Some(name) = arg.strip_prefix("--") {
            let value = args
                .next()
                .ok_or_else(|| bad(format!("--{name} needs a value")))?;
            options.set(name, value)?;
        } else {
            positional.push(arg);
        }
    }
    match command.as_str() {
        "keygen" => {
            let [dir] = take_positional::<1>(positional)?;
            options.none_left()?;
            Ok(Command::Keygen { dir: dir.into() })
        }
        "publish" => {
            let [] = take_positional::<0>(positional)?;
            let publish = Publish {
                key: options.required("key")?.into(),
                feed: options.required("feed")?.into(),
                channel: parse_value("channel", &options.required("channel")?)?,
                label: parse_value("label", &options.required("label")?)?,
                notes: options.notes.take().map(PathBuf::from),
                published: options.published.take(),
                artifacts: with_companions(
                    &std::mem::take(&mut options.artifacts),
                    &std::mem::take(&mut options.companions),
                )?,
                gzip: match options.compressed.take().as_deref() {
                    None => false,
                    Some("gzip") => true,
                    Some(other) => {
                        return Err(bad(format!(
                            "--compressed {other:?}: only gzip is made and read"
                        )));
                    }
                },
            };
            options.none_left()?;
            if publish.artifacts.is_empty() {
                return Err(bad("publish needs at least one --artifact"));
            }
            Ok(Command::Publish(publish))
        }
        "sums" => {
            let [dir] = take_positional::<1>(positional)?;
            let key = options.required("key")?.into();
            let label = parse_value("label", &options.required("label")?)?;
            options.none_left()?;
            Ok(Command::Sums {
                key,
                label,
                dir: dir.into(),
            })
        }
        "check" => {
            let [] = take_positional::<0>(positional)?;
            let command = Command::Check {
                feed: options.required("feed")?.into(),
                public: options.required("public")?,
                channel: parse_value("channel", &options.required("channel")?)?,
            };
            options.none_left()?;
            Ok(command)
        }
        "check-sums" => {
            let [dir] = take_positional::<1>(positional)?;
            let public = options.required("public")?;
            options.none_left()?;
            Ok(Command::CheckSums {
                public,
                dir: dir.into(),
            })
        }
        other => Err(bad(format!("unknown command {other:?}"))),
    }
}

#[derive(Debug, Default)]
struct Options {
    single: Vec<(String, String)>,
    notes: Option<String>,
    published: Option<String>,
    artifacts: Vec<String>,
    companions: Vec<String>,
    compressed: Option<String>,
}

impl Options {
    fn set(&mut self, name: &str, value: String) -> Result<(), ArgsError> {
        match name {
            "artifact" => self.artifacts.push(value),
            "companion" => self.companions.push(value),
            "compressed" => self.compressed = Some(value),
            "notes" => self.notes = Some(value),
            "published" => self.published = Some(value),
            "key" | "feed" | "channel" | "label" | "public" => {
                if self.single.iter().any(|(seen, _)| seen == name) {
                    return Err(bad(format!("--{name} given twice")));
                }
                self.single.push((name.to_owned(), value));
            }
            other => return Err(bad(format!("unknown option --{other}"))),
        }
        Ok(())
    }

    fn required(&mut self, name: &str) -> Result<String, ArgsError> {
        let index = self
            .single
            .iter()
            .position(|(seen, _)| seen == name)
            .ok_or_else(|| bad(format!("--{name} is required")))?;
        Ok(self.single.swap_remove(index).1)
    }

    fn none_left(&self) -> Result<(), ArgsError> {
        let unused = self
            .single
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(self.notes.as_ref().map(|_| "notes"))
            .chain(self.published.as_ref().map(|_| "published"))
            .chain((!self.artifacts.is_empty()).then_some("artifact"))
            .chain((!self.companions.is_empty()).then_some("companion"))
            .chain(self.compressed.as_ref().map(|_| "compressed"))
            .next();
        match unused {
            Some(name) => Err(bad(format!("--{name} does not apply to this command"))),
            None => Ok(()),
        }
    }
}

fn take_positional<const N: usize>(positional: Vec<String>) -> Result<[String; N], ArgsError> {
    positional.try_into().map_err(|given: Vec<String>| {
        bad(format!("expected {N} plain arguments, got {}", given.len()))
    })
}

fn parse_value<T: std::str::FromStr>(name: &str, text: &str) -> Result<T, ArgsError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error: T::Err| bad(format!("--{name} {text:?}: {error}")))
}

fn artifact(text: &str) -> Result<ArtifactArg, ArgsError> {
    let mut parts = text.splitn(3, ':');
    let (Some(target), Some(packaging), Some(path)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(bad(format!(
            "--artifact {text:?} is not TARGET:PACKAGING:PATH"
        )));
    };
    Ok(ArtifactArg {
        target: parse_value("artifact target", target)?,
        packaging: parse_value("artifact packaging", packaging)?,
        path: parse_value("artifact path", path)?,
        companions: Vec::new(),
    })
}

/// The `--artifact` options, each with the `--companion` options naming
/// its target and packaging. A companion with no such artifact, or a role
/// given twice for one, is refused here, before anything is measured.
fn with_companions(
    artifacts: &[String],
    companions: &[String],
) -> Result<Vec<ArtifactArg>, ArgsError> {
    let mut artifacts = artifacts
        .iter()
        .map(|text| artifact(text))
        .collect::<Result<Vec<_>, _>>()?;
    for text in companions {
        let mut parts = text.splitn(4, ':');
        let (Some(target), Some(packaging), Some(role), Some(path)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(bad(format!(
                "--companion {text:?} is not TARGET:PACKAGING:ROLE:PATH"
            )));
        };
        let target: Target = parse_value("companion target", target)?;
        let packaging: Packaging = parse_value("companion packaging", packaging)?;
        let role: Role = parse_value("companion role", role)?;
        let path: ArtifactPath = parse_value("companion path", path)?;
        let owner = artifacts
            .iter_mut()
            .find(|artifact| artifact.target == target && artifact.packaging == packaging)
            .ok_or_else(|| {
                bad(format!(
                    "--companion {text:?} names no --artifact of {target} {packaging}"
                ))
            })?;
        if owner.companions.iter().any(|known| known.role == role) {
            return Err(bad(format!(
                "--companion {role} given twice for {target} {packaging}"
            )));
        }
        owner.companions.push(CompanionArg { role, path });
    }
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> Result<Command, ArgsError> {
        parse(line.split_whitespace().map(OsString::from))
    }

    #[test]
    fn publish_reads_every_option() {
        let command = run(
            "publish --key k --feed f --channel preview --label 0.0.0.0.2 \
             --artifact windows-x86_64:executable:0.0.0.0.2/vessel.exe \
             --artifact linux-x86_64:appimage:0.0.0.0.2/Vessel.AppImage --notes n",
        )
        .unwrap();
        let Command::Publish(publish) = command else {
            panic!("{command:?}");
        };
        assert_eq!(publish.artifacts.len(), 2);
        assert_eq!(publish.artifacts[1].packaging, Packaging::AppImage);
        assert_eq!(publish.notes, Some(PathBuf::from("n")));
        assert!(
            publish
                .artifacts
                .iter()
                .all(|artifact| artifact.companions.is_empty())
        );
        assert!(!publish.gzip);
    }

    #[test]
    fn companions_join_the_artifact_of_their_target_and_packaging() {
        let command = run(
            "publish --key k --feed f --channel preview --label 0.0.0.0.20 \
             --artifact windows-x86_64:executable:0.0.0.0.20/app.exe \
             --artifact linux-x86_64:executable:0.0.0.0.20/app \
             --companion linux-x86_64:executable:node:0.0.0.0.20/vesseld \
             --companion windows-x86_64:executable:cli:0.0.0.0.20/vessel.exe \
             --companion linux-x86_64:executable:cli:0.0.0.0.20/vessel",
        )
        .unwrap();
        let Command::Publish(publish) = command else {
            panic!("{command:?}");
        };
        let roles = |index: usize| -> Vec<Role> {
            publish.artifacts[index]
                .companions
                .iter()
                .map(|companion| companion.role.clone())
                .collect()
        };
        assert_eq!(roles(0), [Role::Cli]);
        assert_eq!(roles(1), [Role::Node, Role::Cli]);
        assert_eq!(
            publish.artifacts[1].companions[0].path.as_str(),
            "0.0.0.0.20/vesseld"
        );
    }

    #[test]
    fn what_is_refused() {
        for line in [
            "",
            "sign x",
            "keygen",
            "keygen a b",
            "keygen a --key k",
            "publish --key k --feed f --channel preview --label 0.0.0.0.2",
            "publish --key k --key k2 --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a",
            "publish --key k --feed f --channel Preview --label 1 --artifact linux-x86_64:executable:a",
            "publish --key k --feed f --channel preview --label 1 --artifact linux:executable:a",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:../a",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a --companion windows-x86_64:executable:cli:b",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a --companion linux-x86_64:executable:cli:b --companion linux-x86_64:executable:cli:c",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a --companion linux-x86_64:executable:b",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a --companion linux-x86_64:executable:Cli:b",
            "check --feed f --public p --channel preview --companion linux-x86_64:executable:cli:b",
            "publish --key k --feed f --channel preview --label 1 --artifact linux-x86_64:executable:a --compressed zstd",
            "check --feed f --public p --channel preview --compressed gzip",
            "check --feed f --public p",
            "sums --key",
            "sums --key k d",
            "sums --key k --label nope d",
        ] {
            assert!(run(line).is_err(), "{line:?} accepted");
        }
    }
}
