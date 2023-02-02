use eframe::egui::Ui;
use crate::command::ModelCommand;
use crate::global_event::GlobalEvent;
use crate::model::{Model, TodoItem};
use crate::ui::view_state::{EditingItem, TodoListViewState, ViewEvent};
use crate::undo_manager::UndoManager;

pub struct ComponentContext<'a> {
    pub model: &'a Model,
    pub undo_manager: &'a UndoManager,
    pub model_commands: &'a mut Vec<ModelCommand>,
    pub global_events: &'a mut Vec<GlobalEvent>,
}

pub struct Component {
    view_state: TodoListViewState,
}

impl Component {
    pub fn new() -> Self {
        Self {
            view_state: TodoListViewState::new(),
        }
    }

    pub fn update(&mut self, ui: &mut Ui, ctx: &mut ComponentContext) {
        let events = self.view_state.update(ui, ctx);

        for event in events {
            match event {
                ViewEvent::TodoItemCreated => {
                    let title = std::mem::take(&mut self.view_state.new_title);
                    ctx.model_commands.push(ModelCommand::CreateTodoItem {
                        id: None,
                        title,
                        completed: false,
                    })
                }
                ViewEvent::EditingStartedTodoItemTitle { id } => {
                    self.view_state.edit_state = Some(EditingItem {
                        id: id,
                        title: ctx.model.items[&id].title.clone(),
                        request_focus: true,
                    });
                }
                ViewEvent::EditingFinishedTodoItemTitle => {
                    self.try_finish_editing(ctx);
                }
                ViewEvent::TodoItemDeleted { id } => {
                    self.try_finish_editing(ctx);
                    ctx.model_commands.push(ModelCommand::DeleteTodoItem { id });
                }
                ViewEvent::TodoItemToggled { id } => {
                    let item = ctx.model.items.get(&id).expect("Can't find with id");
                    ctx.model_commands.push(ModelCommand::UpdateCompletedOfTodoItem {
                        id,
                        completed: !item.completed,
                    });
                }
                ViewEvent::GlobalEvent(e) => {
                    ctx.global_events.push(e);
                }
            }
        }
    }

    fn try_finish_editing(&mut self, ctx: &mut ComponentContext) {
        let Some(EditingItem { id: item_id, title: text_for_edit, .. }) = self.view_state.edit_state.take() else { return; };
        if ctx.model.items[&item_id].title == text_for_edit {
            return;
        }
        ctx.model_commands.push(ModelCommand::UpdateTitleOfTodoItem {
            id: item_id,
            title: text_for_edit,
        });
    }
}
