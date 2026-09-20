//! Política de overlays temporários independente do toolkit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Tooltip,
    Popover,
    ContextMenu,
    FloatingPanel,
    Drawer,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayEntry {
    pub kind: OverlayKind,
    pub pinned: bool,
    pub accepts_outside_click: bool,
}

#[derive(Debug, Default)]
pub struct OverlayStack {
    entries: Vec<OverlayEntry>,
}

impl OverlayStack {
    pub fn push(&mut self, entry: OverlayEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn esc(&mut self) -> Option<OverlayEntry> {
        self.entries.pop()
    }

    pub fn click_away(&mut self) -> Option<OverlayEntry> {
        let index = self
            .entries
            .iter()
            .rposition(|entry| entry.accepts_outside_click && !entry.pinned)?;
        Some(self.entries.remove(index))
    }

    pub fn top(&self) -> Option<OverlayEntry> {
        self.entries.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: OverlayKind) -> OverlayEntry {
        OverlayEntry {
            kind,
            pinned: false,
            accepts_outside_click: true,
        }
    }

    #[test]
    fn escape_closes_the_most_recent_layer_first() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayKind::Drawer));
        stack.push(entry(OverlayKind::Popover));
        assert_eq!(
            stack.esc().map(|item| item.kind),
            Some(OverlayKind::Popover)
        );
        assert_eq!(stack.esc().map(|item| item.kind), Some(OverlayKind::Drawer));
        assert!(stack.is_empty());
    }

    #[test]
    fn outside_click_skips_pinned_layers() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayKind::Drawer));
        stack.push(OverlayEntry {
            kind: OverlayKind::FloatingPanel,
            pinned: true,
            accepts_outside_click: true,
        });
        assert_eq!(
            stack.click_away().map(|item| item.kind),
            Some(OverlayKind::Drawer)
        );
        assert_eq!(
            stack.top().map(|item| item.kind),
            Some(OverlayKind::FloatingPanel)
        );
    }

    #[test]
    fn outside_click_does_not_close_non_dismissible_layers() {
        let mut stack = OverlayStack::default();
        stack.push(OverlayEntry {
            kind: OverlayKind::Modal,
            pinned: false,
            accepts_outside_click: false,
        });
        assert!(stack.click_away().is_none());
        assert_eq!(stack.len(), 1);
    }
}
