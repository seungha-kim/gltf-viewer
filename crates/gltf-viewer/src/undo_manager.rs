use crate::command::TodoListCommand;
use crate::model::TodoListModel;

pub struct UndoManager {
    undo_stack: Vec<TodoListCommand>,
    redo_stack: Vec<TodoListCommand>,
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

    pub fn undo(&mut self, model: &mut TodoListModel) {
        let Some(command) = self.undo_stack.pop() else { return; };
        self.redo_stack.push(model.process_command(command));
    }

    pub fn redo(&mut self, model: &mut TodoListModel) {
        let Some(command) = self.redo_stack.pop() else { return; };
        self.undo_stack.push(model.process_command(command));
    }

    pub fn push_undo(&mut self, command: TodoListCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
    }
}