//! Independent side conversation and composer. Never borrows the main timeline.
//! Existing `ConversationArea`/`Seat` render all message and approval kinds once.

mod actions;
mod draw;
mod intake;

use super::conversation::{ConversationArea, Place, Seat};
use crate::shell::Shell;
use std::cell::RefCell;
use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::side_question::Follow;
use vessel_app_view::side_question::State;

pub(crate) use draw::{beside, toggle};

pub(crate) struct SideQuestion {
    state: State,
    conversation: ConversationArea,
    composer: TextField,
    follow: Option<Follow>,
    sent: Option<String>,
    acknowledged: RefCell<Option<String>>,
    toggle_tip: (vessel_app_kit::Tooltips, vessel_app_kit::tooltip::Place),
}

impl SideQuestion {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            state: State::default(),
            conversation: ConversationArea::new(cx),
            composer: TextField::composer("", Shell::side_submit, window, cx),
            follow: None,
            sent: None,
            acknowledged: RefCell::new(None),
            toggle_tip: (
                vessel_app_kit::Tooltips::new(),
                std::rc::Rc::new(std::cell::Cell::new(None)),
            ),
        }
    }
}

struct SidePlace;
impl Place for SidePlace {
    const SEAT: Seat = SIDE;
    fn busy(shell: &Shell) -> bool {
        shell.areas.side_question.state.pending().is_some()
    }
}

const SIDE: Seat = Seat {
    get: |shell| {
        shell
            .areas
            .side_question
            .state
            .target()
            .map(|_| &shell.areas.side_question.conversation)
    },
    get_mut: |shell| {
        let area = &mut shell.areas.side_question;
        area.state
            .target()
            .is_some()
            .then_some(&mut area.conversation)
    },
};
