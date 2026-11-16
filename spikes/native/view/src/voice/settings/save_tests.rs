//! Held/rejected/lost fake answers exercise the actual Page/form join.
use super::{
    draft::{self, Draft, Input, Provider},
    tests::Node,
    *,
};
use crate::voice::Applied;

struct Fixture {
    node: Node,
    page: Page,
    form: Draft,
    editor: Editor,
    tickets: Tickets,
}
impl Fixture {
    fn new(site: Site) -> Self {
        let node = Node {
            id: NodeId::random(),
            dictation: SetDictation::default(),
            conversation: SetVoice::default(),
        };
        let mut tickets = Tickets::default();
        let read = tickets.next_ticket().unwrap();
        let mut page = Page::reading(node.id, read);
        assert_eq!(
            page.answered(node.id, read, Ok(node.snapshot())),
            Applied::Current
        );
        let editor = page
            .open_editor(page.edit_action(site).unwrap())
            .unwrap()
            .unwrap();
        let mut form = page.editor_draft(editor).unwrap();
        form.edit(Input::Provider(Provider::Endpoint)).unwrap();
        Self {
            node,
            page,
            form,
            editor,
            tickets,
        }
    }
    fn begin(&mut self) -> Write {
        self.page
            .submit_editor(&mut self.form, self.tickets.next_ticket().unwrap())
            .unwrap()
    }
    fn answer(&mut self, request: &Write, reply: Result<Snapshot, Problem>) -> Applied {
        self.page
            .editor_answered(&mut self.form, request.node, request.ticket, reply)
    }
}

#[test]
fn held_write_freezes_edits_and_refusal_preserves_each_site_for_one_fresh_retry() {
    for site in [Site::Dictation, Site::Recognition, Site::Synthesis] {
        let mut f = Fixture::new(site);
        let before = f.node.snapshot();
        let candidate = f.form.change();
        let held = f.begin();
        assert!(f.form.saving());
        assert_eq!(
            f.form.edit(Input::Provider(Provider::Local)),
            Err(draft::Problem::Saving)
        );
        assert!(
            f.page
                .submit_editor(&mut f.form, f.tickets.next_ticket().unwrap())
                .is_none()
        );
        assert_eq!(
            f.page.editor_answered(
                &mut f.form,
                NodeId::random(),
                held.ticket,
                Ok(before.clone())
            ),
            Applied::Stale
        );
        assert!(f.form.saving());
        assert_eq!(
            f.answer(&held, Err(Problem::Refused(ErrorCode::InvalidRequest))),
            Applied::Current
        );
        assert!(!f.form.saving());
        assert_eq!(f.form.change(), candidate);
        assert_eq!(f.node.snapshot(), before);
        assert!(f.page.editor_values(f.editor).is_some());
        let retry = f.begin();
        assert_ne!(held.ticket, retry.ticket);
        let saved = f.node.apply(retry.clone()).unwrap();
        assert_eq!(f.answer(&retry, Ok(saved)), Applied::Current);
        assert!(f.page.editor_values(f.editor).is_none());
        assert!(!f.form.can_save());
        assert_eq!(
            f.form.edit(Input::Provider(Provider::Local)),
            Err(draft::Problem::Saved)
        );
    }
}

#[test]
fn another_opening_or_host_cannot_submit_or_settle_a_retained_draft() {
    let mut f = Fixture::new(Site::Recognition);
    assert_eq!(f.page.close_editor(f.editor), Applied::Current);
    let next = f
        .page
        .open_editor(f.page.edit_action(Site::Recognition).unwrap())
        .unwrap()
        .unwrap();
    let mut current = f.page.editor_draft(next).unwrap();
    current.edit(Input::Provider(Provider::Endpoint)).unwrap();
    assert!(
        f.page
            .submit_editor(&mut f.form, f.tickets.next_ticket().unwrap())
            .is_none()
    );
    let request = f
        .page
        .submit_editor(&mut current, f.tickets.next_ticket().unwrap())
        .unwrap();
    let saved = f.node.apply(request.clone()).unwrap();
    assert_eq!(
        f.page
            .editor_answered(&mut f.form, request.node, request.ticket, Ok(saved.clone())),
        Applied::Stale
    );
    assert!(f.page.editor_values(next).is_some());
    assert_eq!(
        f.page
            .editor_answered(&mut current, request.node, request.ticket, Ok(saved)),
        Applied::Current
    );
    let mut other = Fixture::new(Site::Recognition);
    assert!(
        other
            .page
            .submit_editor(&mut f.form, other.tickets.next_ticket().unwrap())
            .is_none()
    );
}

#[test]
fn lost_reply_is_unconfirmed_even_if_the_fake_server_applied_the_change() {
    let mut f = Fixture::new(Site::Synthesis);
    let before = f.node.snapshot();
    let request = f.begin();
    let actually_saved = f.node.apply(request.clone()).unwrap();
    assert_eq!(
        f.answer(&request, Err(Problem::Unconfirmed)),
        Applied::Current
    );
    let Status::Ready(confirmed, Editing::Idle, Some(Problem::Unconfirmed)) = f.page.status()
    else {
        panic!("unknown outcome");
    };
    assert_eq!(confirmed, &before);
    assert_ne!(confirmed, &actually_saved);
    assert!(!f.page.can_submit_editor(&f.form));
    assert_eq!(
        f.answer(&request, Ok(actually_saved.clone())),
        Applied::Stale
    );
    assert_eq!(f.page.close_editor(f.editor), Applied::Current);
    assert!(f.page.edit_action(Site::Synthesis).is_none());
    let read = f.tickets.next_ticket().unwrap();
    f.page = Page::reading(f.node.id, read);
    assert_eq!(
        f.page.answered(f.node.id, read, Ok(actually_saved)),
        Applied::Current
    );
    assert!(f.page.edit_action(Site::Synthesis).is_some());
    assert!(
        f.page
            .submit_editor(&mut f.form, f.tickets.next_ticket().unwrap())
            .is_none()
    );
}

#[test]
fn dismissing_a_held_form_does_not_cancel_or_reopen_it_and_old_completion_cannot_close_the_next() {
    let mut f = Fixture::new(Site::Dictation);
    let request = f.begin();
    assert_eq!(f.page.close_editor(f.editor), Applied::Current);
    assert!(f.page.edit_action(Site::Dictation).is_none());
    let saved = f.node.apply(request.clone()).unwrap();
    assert_eq!(f.answer(&request, Ok(saved.clone())), Applied::Current);
    assert!(f.page.editor_values(f.editor).is_none());
    let next = f
        .page
        .open_editor(f.page.edit_action(Site::Dictation).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(f.answer(&request, Ok(saved)), Applied::Stale);
    assert!(f.page.editor_values(next).is_some());
}
