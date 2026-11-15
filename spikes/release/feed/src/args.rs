//! The command line, parsed into typed commands. Hand-written: four
//! commands with a few options each do not need an argument parser in the
//! release tool's tree.

use std::{ffi::OsString, path::PathBuf};

use vessel_app_updater::{ArtifactPath, Channel, Label, Packaging, Target};

pub(crate) const USAGE: &str = "\
usage:
  vessel-app-feed keygen DIR
  vessel-app-feed publish --key FILE --feed DIR --channel NAME --label LABEL
                          [--notes FILE] [--published TEXT]
                          --artifact TARGET:PACKAGING:PATH ...
  vessel-app-feed sums --key FILE --label LABEL DIR
  vessel-app-feed check --feed DIR --public KEY --channel NAME
  vessel-app-feed check-sums --public KEY DIR";

/// One artifact named on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactArg {
    pub(crate) target: Target,
    pub(crate) packaging: Packaging,
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
                artifacts: std::mem::take(&mut options.artifacts)
                    .iter()
                    .map(|text| artifact(text))
                    .collect::<Result<_, _>>()?,
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
}

impl Options {
    fn set(&mut self, name: &str, value: String) -> Result<(), ArgsError> {
        match name {
            "artifact" => self.artifacts.push(value),
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
    })
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
            "check --feed f --public p",
            "sums --key",
            "sums --key k d",
            "sums --key k --label nope d",
        ] {
            assert!(run(line).is_err(), "{line:?} accepted");
        }
    }
}
