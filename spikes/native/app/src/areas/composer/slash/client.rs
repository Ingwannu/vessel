//! Exact client tokens are resolved before attachments or optimistic delivery.
//! Lifecycle owns the archive acknowledgement; this adapter never clears text,
//! except after a plugin command, which Paseo's composer empties at once.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::composer::drafts::DraftKey;
use vessel_app_view::composer_shell::catalog::{Entry, plugin_call};
use vessel_app_view::composer_shell::commands::{ClientCommand, Context as CommandContext};
use vessel_app_view::composer_shell::surface::{Access, Mode};
use vessel_app_view::plugins::PluginSlash;

use crate::{routes::Route, shell::Shell};

impl Shell {
    /// True means this submission belongs to the client, including a lifecycle refusal.
    /// Retained context and files always keep a slash token in the provider prompt.
    pub(crate) fn composer_client_command(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(open) = &self.areas.session.open else {
            return false;
        };
        let target = DraftKey::Session {
            session: open.session,
        };
        let surface = self.areas.composer_shell.surface(target);
        if self.composer_current_target() != Some(target)
            || self.route != Route::Session
            || !open.structured
            || surface.mode() != Mode::Chat
            || surface.access() != Access::Editable
            || !self.areas.session.composer.composer_is_editable(cx)
        {
            return false;
        }
        let context = if self.composer_has_files(target) || self.composer_has_context(target) {
            CommandContext::Attached
        } else {
            CommandContext::TextOnly
        };
        let Some(command) = ClientCommand::parse(text, context) else {
            if !self.composer_plugin_command(text, context, window, cx) {
                return false;
            }
            self.areas.composer_shell.collapse(target);
            return true;
        };
        self.areas.composer_shell.collapse(target);
        match command {
            ClientCommand::Exit => self.composer_exit(window, cx),
            ClientCommand::Clear => self.composer_fresh(window, cx),
        }
        true
    }

    /// A submitted `/name args` naming one of the session machine's plugin
    /// commands runs its method instead of going to the agent (Paseo's
    /// `resolvePluginClientSlashCommand`, ADR 0115): the composer empties
    /// at once and a failure is a toast. With files or context attached the
    /// text goes to the agent, as Paseo's.
    fn composer_plugin_command(
        &mut self,
        text: &str,
        context: CommandContext,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let place = self.plugin_place();
        let commands = self.session_plugin_slash();
        let entries: Vec<Entry<'_, PluginSlash>> = commands
            .iter()
            .map(|command| Entry::plugin(&command.name, *command))
            .collect();
        let call = plugin_call(text, context, &entries)
            .and_then(|(command, arguments)| command.call(arguments.as_str(), &place));
        let Some(call) = call else {
            return false;
        };
        self.run_plugin_call(call);
        self.session_composer_sent(window, cx);
        true
    }
}

#[cfg(test)]
mod tests;
