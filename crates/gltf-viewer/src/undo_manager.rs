use crate::command::ModelCommand;
use crate::model::Model;

pub struct UndoManager {
    undo_stack: Vec<ModelCommand>,
    redo_stack: Vec<ModelCommand>,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, model: &mut Model) {
        let Some(command) = self.undo_stack.pop() else { return; };
        self.redo_stack.push(command.mutate(model));
    }

    pub fn redo(&mut self, model: &mut Model) {
        let Some(command) = self.redo_stack.pop() else { return; };
        self.undo_stack.push(command.mutate(model));
    }

    pub fn push_undo(&mut self, command: ModelCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
    }
}