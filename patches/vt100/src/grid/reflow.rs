//! Reflow layout follows headless xterm's viewport/saved-cursor accounting.
//! In particular, output below the cursor can move that cursor while shrinking;
//! holding its original physical row for the whole operation would lose output.
use super::{Grid, Size};
use crate::row::Row;
use alloc::vec::Vec;

struct Layout {
    rows: Vec<Row>,
    markers: super::markers::Markers,
    original_len: usize,
    base: usize,
    cursor: usize,
    saved: usize,
    height: usize,
    limit: usize,
}
impl Grid {
    pub(super) fn reflow_primary<F>(&mut self, size: Size, mut on_row: F)
    where
        F: FnMut(&Row),
    {
        let base = self.scrollback_len();
        let mut rows: Vec<_> = self
            .scrollback
            .as_mut()
            .map(|state| state.rows.drain(..).collect())
            .unwrap_or_default();
        rows.append(&mut self.rows);
        let mut layout = Layout {
            original_len: rows.len(),
            markers: core::mem::take(&mut self.markers),
            rows,
            base,
            cursor: usize::from(self.pos.row),
            saved: base.saturating_add(usize::from(self.saved_pos.row)),
            height: usize::from(size.rows()),
            limit: self
                .scrollback_capacity()
                .saturating_add(usize::from(size.rows())),
        };
        if size.cols() > self.size.cols() {
            layout.widen(self.size.cols(), size.cols());
        } else {
            layout.narrow(self.size.cols(), size.cols());
        }
        let trimmed = layout.rows.len().saturating_sub(layout.limit);
        layout.markers.trim(trimmed);
        for row in layout.rows.drain(..trimmed) {
            // Drain once so a large trim stays linear in the buffer length.
            on_row(&row);
        }
        self.finish_reflow(layout, size);
    }

    fn finish_reflow(&mut self, mut layout: Layout, size: Size) {
        layout.base = layout
            .base
            .min(layout.rows.len().saturating_sub(layout.height));
        // Primary history has no latent right tail after a width shrink.
        // Preserve soft-wrap identity: Row::truncate also serves editing
        // operations whose different contract deliberately clears it.
        for row in &mut layout.rows {
            if row.cols() > size.cols() {
                let wrapped = row.wrapped();
                row.truncate(size.cols());
                row.wrap(wrapped);
            }
        }
        layout.markers.delete(
            layout.base.saturating_add(layout.height),
            layout
                .rows
                .len()
                .saturating_sub(layout.base.saturating_add(layout.height)),
        );
        self.rows = layout.rows.split_off(layout.base);
        self.rows.truncate(layout.height);
        for row in &mut self.rows {
            let wrapped = row.wrapped();
            row.resize(size.cols(), crate::Cell::new());
            row.wrap(wrapped);
        }
        self.rows
            .resize_with(layout.height, || Row::new(size.cols()));
        if let Some(state) = &mut self.scrollback {
            state.rows = layout.rows.into();
            state.offset = 0;
            state.unbounded_len = state.rows.len();
        }
        self.pos.row = u16::try_from(layout.cursor)
            .unwrap_or(u16::MAX)
            .min(size.rows().saturating_sub(1));
        self.saved_pos.row =
            u16::try_from(layout.saved.saturating_sub(layout.base))
                .unwrap_or(u16::MAX)
                .min(size.rows().saturating_sub(1));
        self.pos.col = self.pos.col.min(size.cols().saturating_sub(1));
        self.saved_pos.col =
            self.saved_pos.col.min(size.cols().saturating_sub(1));
        self.scroll_top = 0;
        self.scroll_bottom = size.rows().saturating_sub(1);
        self.size = size;
        self.markers = layout.markers;
    }
}
impl Layout {
    fn widen(&mut self, old_cols: u16, cols: u16) {
        let old_len = self.rows.len();
        let absolute_cursor = self.base.saturating_add(self.cursor);
        let (rows, mapping) = crate::row::reflow_rows(
            core::mem::take(&mut self.rows),
            Some(absolute_cursor),
            old_cols,
            cols,
        );
        self.rows = rows;
        self.markers.remap(&mapping);
        let removed = old_len.saturating_sub(self.rows.len());
        let from_base = removed.min(self.base);
        self.base = self.base.saturating_sub(from_base);
        self.cursor = self
            .cursor
            .saturating_sub(removed.saturating_sub(from_base));
        self.saved = self.saved.saturating_sub(removed);
        self.rows
            .resize_with(self.rows.len().max(self.height), || Row::new(cols));
    }

    fn narrow(&mut self, old_cols: u16, cols: u16) {
        let mut inserts = Vec::new();
        let mut end = self.rows.len();
        let mut added = 0_usize;
        while end > 0 {
            let last = end.saturating_sub(1);
            if self.rows.get(last).is_none() {
                end = last;
                continue;
            }
            let mut start = last;
            while start > 0
                && self
                    .rows
                    .get(start.saturating_sub(1))
                    .is_some_and(Row::wrapped)
            {
                start = start.saturating_sub(1);
            }
            let cursor = self.base.saturating_add(self.cursor);
            let needs_reflow = start != last
                || self.rows.get(last).is_some_and(|row| {
                    (cols..row.cols()).any(|col| {
                        row.get(col).is_some_and(|cell| {
                            cell.has_contents() || cell.is_wide_continuation()
                        })
                    })
                });
            if needs_reflow && (cursor < start || cursor >= end) {
                let extra = self.repack(start, end, old_cols, cols);
                let count = extra.len();
                if count > 0 {
                    inserts.push((end, extra));
                    added = added.saturating_add(count);
                    self.adjust_narrow(count, added);
                }
            }
            end = start;
        }
        self.merge_insertions(inserts);
    }

    fn merge_insertions(&mut self, mut inserts: Vec<(usize, Vec<Row>)>) {
        // Original rows can be popped below the cursor while insertions are
        // pending. Keep their extra rows in original order even past that tail.
        inserts.sort_by_key(|(at, _)| *at);
        let mut originals = core::mem::take(&mut self.rows)
            .into_iter()
            .enumerate()
            .peekable();
        let mut rows = Vec::new();
        let mut mapping = vec![None; self.original_len];
        for (at, extra) in inserts {
            while originals.peek().is_some_and(|(index, _)| *index < at) {
                if let Some((index, row)) = originals.next() {
                    if let Some(mapped) = mapping.get_mut(index) {
                        *mapped = Some(rows.len());
                    }
                    rows.push(row);
                }
            }
            rows.extend(extra);
        }
        for (index, row) in originals {
            if let Some(mapped) = mapping.get_mut(index) {
                *mapped = Some(rows.len());
            }
            rows.push(row);
        }
        self.rows = rows;
        self.markers.remap(&mapping);
    }

    fn repack(
        &mut self,
        start: usize,
        end: usize,
        old_cols: u16,
        cols: u16,
    ) -> Vec<Row> {
        let mut group = Vec::new();
        if let Some(rows) = self.rows.get_mut(start..end) {
            for row in rows.iter_mut() {
                group.push(core::mem::replace(row, Row::new(0)));
            }
        }
        let (mut packed, _) =
            crate::row::reflow_rows(group, None, old_cols, cols);
        let extra =
            packed.split_off(packed.len().min(end.saturating_sub(start)));
        for (index, row) in packed.into_iter().enumerate() {
            if let Some(old) = self.rows.get_mut(start.saturating_add(index))
            {
                *old = row;
            }
        }
        extra
    }

    fn adjust_narrow(&mut self, count: usize, added: usize) {
        let occupied = if self.base == 0
            && self.cursor != self.rows.len().saturating_sub(1)
        {
            self.cursor
        } else {
            self.rows.len()
        };
        let trimmed =
            occupied.saturating_add(count).saturating_sub(self.limit);
        for _ in 0..count.saturating_sub(trimmed) {
            if self.base == 0 {
                if self.cursor < self.height.saturating_sub(1) {
                    self.cursor = self.cursor.saturating_add(1);
                    drop(self.rows.pop());
                } else {
                    self.base = self.base.saturating_add(1);
                }
            } else if self.base
                < self
                    .limit
                    .min(self.rows.len().saturating_add(added))
                    .saturating_sub(self.height)
            {
                self.base = self.base.saturating_add(1);
            }
        }
        self.saved = self
            .saved
            .saturating_add(count)
            .min(self.base.saturating_add(self.height).saturating_sub(1));
    }
}
