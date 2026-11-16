//! One current provider form joins the existing Page write owner.
//! A candidate never follows mutable machine selection or another opening.
use super::{Edit, Editor, Page, Pending, Problem, Snapshot, Write, draft};
use crate::{devices::Ticket, voice::Applied};
use vessel_proto::NodeId;

impl Page {
    /// Open only the displayed site, giving each form a fresh existing ticket.
    /// The shared dialog remains the sole owner of focus and modal layers.
    ///
    /// # Errors
    /// Exhausting the ticket source refuses a new form instead of reusing one.
    pub fn open_editor(&mut self, action: Edit) -> Result<Option<Editor>, crate::devices::Problem> {
        let Some(site) = self.edit(action) else {
            return Ok(None);
        };
        let editor = Editor {
            node: self.node,
            source: self.ticket,
            opening: self.editor_tickets.next_ticket()?,
            site,
        };
        self.editor = Some(editor);
        Ok(Some(editor))
    }

    /// Read only the confirmed metadata for this still-current opening.
    /// Stored key values and reference names are absent from the snapshot.
    #[must_use]
    pub fn editor_values(&self, editor: Editor) -> Option<&Snapshot> {
        if self.editor != Some(editor) || editor.node != self.node {
            return None;
        }
        self.read.as_ref()?.as_ref().ok()
    }

    /// Create a non-secret form model only for this current editor opening.
    #[must_use]
    pub fn editor_draft(&self, editor: Editor) -> Option<draft::Draft> {
        draft::Draft::new(editor, self.editor_values(editor)?)
    }

    /// An old close/cancel callback must not dismiss the next form.
    pub fn close_editor(&mut self, editor: Editor) -> Applied {
        if self.editor_values(editor).is_none() {
            return Applied::Stale;
        }
        self.editor = None;
        Applied::Current
    }

    /// Recheck the original form, node, pending state and known outcome.
    #[must_use]
    pub fn can_submit_editor(&self, form: &draft::Draft) -> bool {
        self.pending == Pending::Settled
            && !self.save_blocked()
            && self.editor_values(form.editor()).is_some()
            && form.can_save()
    }

    /// Atomically freeze this form and reserve one node-bound candidate write.
    /// No transport or speculative node settings field is introduced here.
    pub fn submit_editor(&mut self, form: &mut draft::Draft, ticket: Ticket) -> Option<Write> {
        if ticket == self.ticket || !self.can_submit_editor(form) {
            return None;
        }
        let change = form.change()?;
        let editor = form.editor();
        form.begin(ticket);
        self.ticket = ticket;
        self.pending = Pending::EditorWrite(editor);
        self.failure = None;
        Some(Write {
            node: self.node,
            ticket,
            change,
        })
    }

    /// Join the actual answer only to its submitted form and operation.
    /// Refusal unlocks that draft; success closes only the same opening.
    pub fn editor_answered(
        &mut self,
        form: &mut draft::Draft,
        node: NodeId,
        ticket: Ticket,
        reply: Result<Snapshot, Problem>,
    ) -> Applied {
        if self.pending != Pending::EditorWrite(form.editor()) || !form.pending(ticket) {
            return Applied::Stale;
        }
        let accepted = reply.is_ok();
        let applied = self.answered(node, ticket, reply);
        if applied == Applied::Current {
            form.finished(accepted);
        }
        applied
    }
}
