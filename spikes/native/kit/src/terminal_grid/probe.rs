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
pub(super) enum Stage {
    Prepare,
    Paint,
}
#[derive(Default, Clone)]
pub(super) struct DrawProbe(Rc<Cell<Counts>>);
impl DrawProbe {
    pub(super) fn note(&self, stage: Stage) {
        let mut counts = self.0.get();
        match stage {
            Stage::Prepare => counts.prepared = counts.prepared.next(),
            Stage::Paint => counts.painted = counts.painted.next(),
        }
        self.0.set(counts);
    }
    pub(super) fn counts(&self) -> Counts {
        self.0.get()
    }
}
