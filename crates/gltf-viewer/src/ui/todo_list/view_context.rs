use crate::command::TodoListModelCommand;
use crate::model::TodoListModel;
use crate::ui::framework;
use crate::undo_manager::UndoManager;

pub struct TodoListViewContext<'a> {
    model: &'a TodoListModel,
    undo_manager: &'a UndoManager,
    model_commands: &'a mut Vec<TodoListModelCommand>,
    undo_requested: bool,
    redo_requested: bool,
    exit_requested: bool,
}

impl<'a> TodoListViewContext<'a> {
    pub fn new(
        model: &'a TodoListModel,
        undo_manager: &'a UndoManager,
        model_commands: &'a mut Vec<TodoListModelCommand>,
    ) -> Self {
        Self {
            model,
            undo_manager,
            model_commands,
            undo_requested: false,
            redo_requested: false,
            exit_requested: false,
        }
    }
}

impl framework::ViewContext for TodoListViewContext<'_> {
    type Model = TodoListModel;
    type Command = TodoListModelCommand;

    fn model(&self) -> &Self::Model {
        self.model
    }

    fn can_undo(&self) -> bool {
        self.undo_manager.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.undo_manager.can_redo()
    }

    fn undo_requested(&self) -> bool {
        self.undo_requested
    }

    fn redo_requested(&self) -> bool {
        self.redo_requested
    }

    fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    fn request_undo(&mut self) {
        self.undo_requested = true;
    }

    fn request_redo(&mut self) {
        self.redo_requested = true;
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn push_command(&mut self, command: Self::Command) {
        self.model_commands.push(command);
    }
}
