//! Test-only observation of real canvas callbacks, never a product rendering switch.
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct DrawCount(u32);
impl DrawCount {
    fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("bounded test frames"))
    }
    pub(super) fn get(self) -> u32 {
        self.0
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Counts {
    pub(super) prepared: DrawCount,
    pub(super) painted: DrawCount,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum Stage {
    Prepare,
    Paint,
    Cursor,
}
#[derive(Debug, Default, Clone)]
pub(super) struct DrawProbe {
    rows: Rc<Cell<Counts>>,
    cursor: Rc<Cell<DrawCount>>,
}
impl DrawProbe {
    pub(super) fn note(&self, stage: Stage) {
        let mut counts = self.rows.get();
        match stage {
            Stage::Prepare => counts.prepared = counts.prepared.next(),
            Stage::Paint => counts.painted = counts.painted.next(),
            Stage::Cursor => self.cursor.set(self.cursor.get().next()),
        }
        self.rows.set(counts);
    }
    pub(super) fn counts(&self) -> Counts {
        self.rows.get()
    }
    pub(super) fn cursor(&self) -> DrawCount {
        self.cursor.get()
    }
}
