//! Script approval wording shares one field comparison so a declared port
//! cannot disappear from one language while the other still names it.

use vessel_proto::scripts::{ScriptEntry, ScriptKind};
use vessel_proto::workspace::ServicePort;

use crate::Lang;

pub(super) const fn kind(language: Lang, kind: ScriptKind) -> &'static str {
    match (language, kind) {
        (Lang::En, ScriptKind::Script) => "script",
        (Lang::En, ScriptKind::Service) => "service",
        (Lang::Ko, ScriptKind::Script) => "스크립트",
        (Lang::Ko, ScriptKind::Service) => "서비스",
    }
}

pub(super) fn port(language: Lang, port: Option<ServicePort>) -> String {
    match (language, port) {
        (Lang::En, Some(port)) => format!("port {port}"),
        (Lang::En, None) => "no declared port".to_owned(),
        (Lang::Ko, Some(port)) => format!("포트 {port}"),
        (Lang::Ko, None) => "지정 포트 없음".to_owned(),
    }
}

pub(super) fn change(language: Lang, was: &ScriptEntry, now: &ScriptEntry) -> String {
    let (command, folder, join) = match language {
        Lang::En => ("its command", "its folder", " and "),
        Lang::Ko => ("명령", "폴더", ", "),
    };
    let mut parts = Vec::new();
    if was.kind() != now.kind() {
        let kind = kind(language, now.kind());
        parts.push(match language {
            Lang::En => format!("now a {kind}"),
            Lang::Ko => format!("이제 {kind}"),
        });
    }
    if was.command != now.command {
        parts.push(command.to_owned());
    }
    if was.dir != now.dir {
        parts.push(folder.to_owned());
    }
    if was.port() != now.port() {
        parts.push(format!(
            "{} → {}",
            port(language, was.port()),
            port(language, now.port())
        ));
    }
    parts.join(join)
}
