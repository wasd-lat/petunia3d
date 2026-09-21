//! Petunia3D — Undo/Redo via Command Pattern (snapshots).
//!
//! Destructive operations are undoable commands. Snapshots of the affected
//! document are acceptable for V1. History is bounded by a **byte budget**
//! (default 256 MiB), discarding oldest entries first — not a fixed op count.

/// Default history memory budget (ch. 16): 256 MiB per document.
pub const DEFAULT_HISTORY_BUDGET_BYTES: usize = 256 * 1024 * 1024;

/// Snapshot of history memory use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryMetrics {
    pub history_entries: usize,
    pub history_bytes: usize,
    pub largest_entry: usize,
    pub average_entry: usize,
}

#[derive(Debug, Clone)]
struct HistoryEntry<T> {
    label: String,
    value: T,
    bytes: usize,
}

/// Pilha genérica de undo/redo sobre estado clonável com rastreamento determinístico de estado salvo/dirty.
#[derive(Debug)]
pub struct UndoStack<T: Clone> {
    undo: Vec<HistoryEntry<T>>,
    redo: Vec<HistoryEntry<T>>,
    cap: usize,
    byte_budget: usize,
    undo_bytes: usize,
    redo_bytes: usize,
    clean_version: Option<usize>,
    current_version: usize,
}

impl<T: Clone> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> UndoStack<T> {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            cap: 4096,
            byte_budget: DEFAULT_HISTORY_BUDGET_BYTES,
            undo_bytes: 0,
            redo_bytes: 0,
            clean_version: Some(0),
            current_version: 0,
        }
    }

    pub fn with_budget(byte_budget: usize) -> Self {
        let mut s = Self::new();
        s.byte_budget = byte_budget.max(1);
        s
    }

    pub fn set_byte_budget(&mut self, bytes: usize) {
        self.byte_budget = bytes.max(1);
        self.evict_to_budget();
    }

    /// Salva o estado ATUAL antes de uma mutação (chamar antes de mudar).
    pub fn checkpoint(&mut self, label: impl Into<String>, current: &T) {
        self.checkpoint_sized(label, current, estimate_bytes(current));
    }

    /// Checkpoint with an explicit payload size (preferred for `Project`).
    pub fn checkpoint_sized(&mut self, label: impl Into<String>, current: &T, bytes: usize) {
        let bytes = bytes.max(1);
        self.undo.push(HistoryEntry {
            label: label.into(),
            value: current.clone(),
            bytes,
        });
        self.undo_bytes = self.undo_bytes.saturating_add(bytes);
        self.current_version = self.current_version.saturating_add(1);
        self.redo_bytes = 0;
        self.redo.clear();
        self.evict_to_budget();
        while self.undo.len() > self.cap {
            if let Some(old) = self.undo.first() {
                self.undo_bytes = self.undo_bytes.saturating_sub(old.bytes);
            }
            self.undo.remove(0);
        }
    }

    fn evict_to_budget(&mut self) {
        while self.undo_bytes > self.byte_budget && self.undo.len() > 1 {
            let old = self.undo.remove(0);
            self.undo_bytes = self.undo_bytes.saturating_sub(old.bytes);
        }
    }

    pub fn metrics(&self) -> HistoryMetrics {
        let entries: Vec<usize> = self
            .undo
            .iter()
            .map(|e| e.bytes)
            .chain(self.redo.iter().map(|e| e.bytes))
            .collect();
        let history_entries = entries.len();
        let history_bytes = self.undo_bytes.saturating_add(self.redo_bytes);
        let largest_entry = entries.iter().copied().max().unwrap_or(0);
        let average_entry = history_bytes.checked_div(history_entries).unwrap_or(0);
        HistoryMetrics {
            history_entries,
            history_bytes,
            largest_entry,
            average_entry,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }
    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    /// Marca o estado atual do histórico como sincronizado/salvo em disco.
    pub fn mark_clean(&mut self) {
        self.clean_version = Some(self.current_version);
    }

    /// Força o estado a ser marcado como não salvo (dirty).
    pub fn mark_dirty(&mut self) {
        self.clean_version = None;
    }

    /// Verifica se o estado atual coincide exatamente com o ponto salvo.
    pub fn is_clean(&self) -> bool {
        self.clean_version == Some(self.current_version)
    }

    /// Verifica se há alterações não salvas no histórico.
    pub fn is_dirty(&self) -> bool {
        !self.is_clean()
    }

    /// Desfaz: guarda estado atual no redo, retorna estado anterior.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.undo_bytes = self.undo_bytes.saturating_sub(prev.bytes);
        self.current_version = self.current_version.saturating_sub(1);
        let current_bytes = estimate_bytes(&current);
        self.redo.push(HistoryEntry {
            label: prev.label,
            value: current,
            bytes: current_bytes,
        });
        self.redo_bytes = self.redo_bytes.saturating_add(current_bytes);
        Some(prev.value)
    }

    /// Refaz: guarda estado atual no undo, retorna próximo estado.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.redo_bytes = self.redo_bytes.saturating_sub(next.bytes);
        self.current_version = self.current_version.saturating_add(1);
        let current_bytes = estimate_bytes(&current);
        self.undo.push(HistoryEntry {
            label: next.label,
            value: current,
            bytes: current_bytes,
        });
        self.undo_bytes = self.undo_bytes.saturating_add(current_bytes);
        Some(next.value)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
        self.current_version = 0;
        self.clean_version = Some(0);
    }
}

fn estimate_bytes<T>(value: &T) -> usize {
    std::mem::size_of_val(value).max(1)
}

/// Trait genérica para comandos executáveis e transacionais.
pub trait Command<Context, Res = (), Err = String>: Send + Sync {
    /// Rótulo legível para telemetria e pilha de desfazer/refazer (Undo/Redo).
    fn label(&self) -> &'static str;

    /// Executa a operação contra o contexto mutável.
    fn execute(&self, ctx: &mut Context) -> Result<Res, Err>;

    /// Indica se o comando altera o estado persistente e exige captura prévia de checkpoint de Undo.
    fn is_destructive(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_roundtrip() {
        let mut st: UndoStack<Vec<i32>> = UndoStack::new();
        let mut cur = vec![1];
        st.checkpoint("add", &cur);
        cur.push(2);
        assert_eq!(st.undo(cur.clone()), Some(vec![1]));
        let cur = vec![1];
        assert_eq!(st.redo(cur), Some(vec![1, 2]));
        assert!(!st.can_redo());
    }

    #[test]
    fn checkpoint_clears_redo() {
        let mut st: UndoStack<i32> = UndoStack::new();
        st.checkpoint("a", &1);
        let _ = st.undo(2);
        assert!(st.can_redo());
        st.checkpoint("b", &1);
        assert!(!st.can_redo());
    }

    struct IncrementCmd(i32);
    impl Command<i32> for IncrementCmd {
        fn label(&self) -> &'static str {
            "increment"
        }
        fn execute(&self, ctx: &mut i32) -> Result<(), String> {
            *ctx += self.0;
            Ok(())
        }
    }

    #[test]
    fn test_undo_stack_dirty_state_tracking() {
        let mut st: UndoStack<i32> = UndoStack::new();
        // Initial state is clean (at saved/start version 0)
        assert!(st.is_clean());
        assert!(!st.is_dirty());

        // Mutation 1 -> dirty
        let mut cur = 0;
        st.checkpoint("step 1", &cur);
        cur = 1;
        assert!(st.is_dirty());

        // Mark saved (clean)
        st.mark_clean();
        assert!(st.is_clean());
        assert!(!st.is_dirty());

        // Mutation 2 -> dirty
        st.checkpoint("step 2", &cur);
        cur = 2;
        assert!(st.is_dirty());

        // Undo -> back to step 1 which was marked clean!
        let prev = st.undo(cur).unwrap();
        assert_eq!(prev, 1);
        assert!(
            st.is_clean(),
            "Undoing back to the saved state must be clean"
        );

        // Redo -> forward to step 2 which is dirty!
        let next = st.redo(prev).unwrap();
        assert_eq!(next, 2);
        assert!(st.is_dirty(), "Redoing to an unsaved state must be dirty");

        // Force dirty
        st.mark_clean();
        assert!(st.is_clean());
        st.mark_dirty();
        assert!(st.is_dirty());
    }

    #[test]
    fn command_trait_executes_and_identifies() {
        let cmd = IncrementCmd(5);
        assert_eq!(cmd.label(), "increment");
        assert!(cmd.is_destructive());
        let mut val = 10;
        assert!(cmd.execute(&mut val).is_ok());
        assert_eq!(val, 15);
    }

    #[test]
    fn byte_budget_evicts_oldest_not_by_count() {
        let mut st: UndoStack<Vec<u8>> = UndoStack::with_budget(64);
        for i in 0..8 {
            let cur = vec![i; 20];
            st.checkpoint_sized("step", &cur, 20);
        }
        let metrics = st.metrics();
        assert!(metrics.history_bytes <= 64);
        assert!(metrics.history_entries <= 4);
        assert!(metrics.largest_entry <= 20);
    }
}
