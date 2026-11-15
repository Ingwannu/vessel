//! Every desktop window follows the single canonical preferences actor.

use gpui::{Context,Window};
use vessel_app_link::{Command,Prefs};
use vessel_app_link::desktop::preferences::{Outcome,Snapshot};
use crate::shell::Shell;
use super::Desktop;

pub(crate) fn start(shell:&mut Shell,window:&mut Window,cx:&mut Context<'_,Shell>) {
    let Some(desktop)=cx.try_global::<Desktop>() else {return;};
    let (client,mut updates)=desktop.preferences.client();
    shell.areas.desktop.preferences=Some(client);
    cx.spawn_in(window,async move |this,cx| {
        while updates.changed().await.is_ok() {
            let snapshot=updates.borrow_and_update().clone();
            if this.update_in(cx,|shell,window,cx|apply(shell,&snapshot,window,cx)).is_err() {break;}
        }
    }).detach();
}

pub(crate) fn submit(shell:&Shell,command:Command) -> Option<Command> {
    let Some(client)=&shell.areas.desktop.preferences else {return Some(command);};
    match command {
        Command::SavePrefs(prefs)=>{
            if let Err(error)=client.publish(prefs) {
                // The existing unsent path settles refused preference writes in
                // the window; do not send them to an independent file writer.
                tracing::warn!(%error,"preferences queue refused a desktop edit");
            }
            None
        }
        other=>Some(other),
    }
}

fn apply(shell:&mut Shell,snapshot:&Snapshot,window:&mut Window,cx:&mut Context<'_,Shell>) {
    let Some(client)=&shell.areas.desktop.preferences else {return;};
    client.observe(snapshot);
    let next=snapshot.preferences();
    if shell.prefs!=*next {
        apply_preferences(shell,(*next).clone(),window,cx);
    }
    if let Outcome::Failed(error)=snapshot.outcome() {
        tracing::warn!(%error,"preferences change was refused");
        shell.notice=Some(shell.words.desktop_shell().cli_failed().to_owned());
    }
    cx.notify();
}

fn apply_preferences(shell:&mut Shell,next:Prefs,window:&mut Window,cx:&mut Context<'_,Shell>) {
    let old=std::mem::replace(&mut shell.prefs,next);
    if old.settings.language!=shell.prefs.settings.language {
        match shell.language.prepare(shell.prefs.settings.language) {
            Ok(words)=>shell.apply_language(words,window,cx),
            Err(error)=>tracing::error!(%error,"shared preferences language could not be prepared"),
        }
    }
    crate::areas::settings::apply_theme(shell,window,cx);
    crate::areas::settings::apply_composer(shell,window,cx);
    crate::areas::settings::apply_text_sizes(shell,cx);
    crate::areas::session::set_content_width(shell.prefs.settings.appearance.content_width);
    shell.conversation_display(shell.prefs.settings.appearance.conversation);
    crate::motion::animations(shell.prefs.settings.appearance.animations.on(),cx);
    if old.manual_updates!=shell.prefs.manual_updates {
        shell.set_update_automatically(!shell.prefs.manual_updates,cx);
    }
    window.refresh();
}
