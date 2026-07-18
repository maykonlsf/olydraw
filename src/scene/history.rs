use super::element::{Element, ElementId};

const MAX_HISTORY: usize = 100;

/// An applied scene mutation, storing enough to invert it.
#[derive(Debug, Clone)]
pub enum Command {
    Add {
        element: Element,
    },
    /// Elements removed with their original indices (ascending).
    Remove {
        removed: Vec<(usize, Element)>,
    },
    Translate {
        ids: Vec<ElementId>,
        dx: f32,
        dy: f32,
    },
    Clear {
        removed: Vec<Element>,
    },
}

#[derive(Default)]
pub struct History {
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl History {
    pub fn push(&mut self, cmd: Command) {
        self.redo.clear();
        self.undo.push(cmd);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
    }

    pub fn pop_undo(&mut self) -> Option<Command> {
        let cmd = self.undo.pop()?;
        self.redo.push(cmd.clone());
        Some(cmd)
    }

    pub fn pop_redo(&mut self) -> Option<Command> {
        let cmd = self.redo.pop()?;
        self.undo.push(cmd.clone());
        Some(cmd)
    }
}
