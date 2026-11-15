use super::*;
use vessel_app_kit::Editability;
use vessel_app_link::Command;
impl Shell {
    pub(in crate::areas::settings::agents) fn agent_open_add(
        &mut self,
        entry: Option<CatalogueEntry>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.areas.settings.agents.machine.clone() else {
            return;
        };
        if !self.agent_context(Some(&machine)) {
            return;
        }
        let state = &mut self.areas.settings.agents;
        let intent = match &entry {
            Some(entry) => Intent::registry(&state.pages, machine.clone(), entry),
            None => Intent::command(&state.pages, machine.clone()),
        };
        let intent = match intent {
            Ok(intent) => Rc::new(intent),
            Err(problem) => {
                state.addition.refused = Some(problem);
                cx.notify();
                return;
            }
        };
        let words = self.words.agent_catalog();
        let name = TextField::with_change(words.name_placeholder(), edit_only, changed, window, cx);
        if let Some(entry) = &entry
            && let Some(suggested) = vessel_app_view::agent_status::addition::suggested_name(entry)
        {
            name.set_text(suggested.as_str(), window, cx);
        }
        let program =
            TextField::with_change(words.program_placeholder(), edit_only, changed, window, cx);
        let arguments =
            TextField::form_multiline(words.arguments_placeholder(), edit_only, window, cx);
        let state = &mut self.areas.settings.agents.addition;
        state.restore.set(Restore::Keep);
        state.refused = None;
        state.dialog.focus(window, cx);
        name.focus(window, cx);
        state.form = Some(Form {
            intent,
            machine,
            registry: entry,
            name,
            program,
            arguments,
            phase: Phase::Editing,
            invalid: None,
        });
        cx.notify();
    }
    pub(in crate::areas::settings::agents) fn agent_close_add(
        &mut self,
        origin: &Rc<Intent>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.agents.addition;
        if state
            .form
            .as_ref()
            .is_some_and(|form| Rc::ptr_eq(origin, &form.intent))
        {
            state.form = None;
            state.refused = None;
            state.restore.set(Restore::Keep);
            state.dialog.dismissed(window, cx);
            cx.notify();
        }
    }
    pub(in crate::areas::settings::agents) fn agent_submit_add(
        &mut self,
        origin: &Rc<Intent>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(machine) = self.areas.settings.agents.machine.clone() else {
            return;
        };
        if !self.agent_context(Some(&machine)) {
            return;
        }
        let state = &mut self.areas.settings.agents;
        let Some(form) = state.addition.form.as_ref().filter(|form| {
            form.machine == machine
                && Rc::ptr_eq(origin, &form.intent)
                && matches!(form.phase, Phase::Editing)
        }) else {
            return;
        };
        let (name, program, arguments) = (
            form.name.text(cx),
            form.program.text(cx),
            form.arguments.text(cx),
        );
        let state = &mut self.areas.settings.agents;
        let Some(form) = state.addition.form.as_mut() else {
            return;
        };
        match form
            .intent
            .submit(&machine, &mut state.pages, &name, &program, &arguments)
        {
            Ok(request) => {
                let ticket = request.ticket();
                form.phase = Phase::Sending(ticket);
                form.invalid = None;
                for field in [&form.name, &form.program, &form.arguments] {
                    field.set_editability(Editability::ReadOnly, cx);
                }
                self.command(Command::add_agent(request));
            }
            Err(error) => form.invalid = Some(error),
        }
        cx.notify();
    }
    pub(in crate::areas::settings::agents) fn agent_add_settled(
        &mut self,
        machine: &MachineKey,
        ticket: RequestId,
        outcome: Settlement,
        cx: &mut Context<'_, Self>,
    ) {
        let state = &mut self.areas.settings.agents.addition;
        let Some(form) = state.form.as_mut().filter(|form| {
            form.machine == *machine && matches!(form.phase,Phase::Sending(sent) if sent==ticket)
        }) else {
            return;
        };
        if matches!(outcome, Settlement::Accepted) {
            state.form = None;
            state.restore.set(Restore::Dismiss);
        } else {
            form.phase = Phase::Editing;
            for field in [&form.name, &form.program, &form.arguments] {
                field.set_editability(Editability::Editable, cx);
            }
        }
    }
}
