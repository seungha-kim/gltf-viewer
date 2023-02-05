use eframe::egui;
use eframe::egui::Ui;
use crate::command::TodoListModelCommand;
use crate::ui::framework::*;
use super::view_context::TodoListViewContext;

pub struct TodoListViewState {
    new_title: String,
    edit_state: Option<EditingItem>,
}

struct EditingItem {
    pub request_focus: bool,
    pub id: uuid::Uuid,
    pub title: String,
}

pub enum TodoListViewEvent {
    TodoItemCreated,
    EditingStartedTodoItemTitle {
        id: uuid::Uuid,
    },
    EditingFinishedTodoItemTitle,
    TodoItemDeleted {
        id: uuid::Uuid,
    },
    TodoItemToggled {
        id: uuid::Uuid,
    },
    UndoRequested,
    RedoRequested,
}

impl ViewState for TodoListViewState {
    type Context<'a> = TodoListViewContext<'a>;
    type Event = TodoListViewEvent;

    fn interact(&mut self, ui: &mut Ui, ctx: &Self::Context<'_>, events: &mut Vec<Self::Event>) {
        ui.heading("To-do List");
        if ui.add_enabled(ctx.can_undo(), egui::widgets::Button::new("Undo")).clicked() {
            events.push(TodoListViewEvent::UndoRequested);
        }
        if ui.add_enabled(ctx.can_redo(), egui::widgets::Button::new("Redo")).clicked() {
            events.push(TodoListViewEvent::RedoRequested);
        }

        self.text_edit(ui, events);
        self.todo_list(ui, ctx, events);

        let input = &ui.ctx().input();
        // NOTE: fizz-buzz!
        if input.modifiers.command && input.modifiers.shift && input.key_pressed(egui::Key::Z) {
            events.push(TodoListViewEvent::RedoRequested);
        } else if input.modifiers.command && input.key_pressed(egui::Key::Z) {
            events.push(TodoListViewEvent::UndoRequested);
        }
    }

    fn handle_view_event(&mut self, ctx: &mut Self::Context<'_>, event: Self::Event) {
        match event {
            TodoListViewEvent::TodoItemCreated => {
                let title = std::mem::take(&mut self.new_title);
                ctx.push_command(TodoListModelCommand::CreateTodoItem {
                    id: None,
                    title,
                    completed: false,
                })
            }
            TodoListViewEvent::EditingStartedTodoItemTitle { id } => {
                self.edit_state = Some(EditingItem {
                    id,
                    title: ctx.model().items[&id].title.clone(),
                    request_focus: true,
                });
            }
            TodoListViewEvent::EditingFinishedTodoItemTitle => {
                self.try_finish_editing(ctx);
            }
            TodoListViewEvent::TodoItemDeleted { id } => {
                self.try_finish_editing(ctx);
                ctx.push_command(TodoListModelCommand::DeleteTodoItem { id });
            }
            TodoListViewEvent::TodoItemToggled { id } => {
                let item = ctx.model().items.get(&id).expect("Can't find with id");
                ctx.push_command(TodoListModelCommand::UpdateCompletedOfTodoItem {
                    id,
                    completed: !item.completed,
                });
            }
            TodoListViewEvent::UndoRequested => {
                ctx.request_undo();
            }
            TodoListViewEvent::RedoRequested => {
                ctx.request_redo();
            }
        }
    }
}

impl TodoListViewState {
    pub fn new() -> Self {
        Self {
            new_title: "".into(),
            edit_state: None,
        }
    }

    fn text_edit(&mut self, ui: &mut egui::Ui, events: &mut Vec<TodoListViewEvent>) {
        // Props
        let is_empty_text = self.new_title.is_empty();

        let (text_edit, add_button) = ui
            .horizontal(|ui| {
                (
                    ui.text_edit_singleline(&mut self.new_title),
                    ui.add_enabled(!is_empty_text, egui::widgets::Button::new("Add")),
                )
            })
            .inner;

        if Self::enter_pressed(&text_edit, ui.ctx()) || add_button.clicked() {
            text_edit.request_focus();
            events.push(TodoListViewEvent::TodoItemCreated);
        }
    }

    fn todo_list(&mut self, ui: &mut egui::Ui, ctx: &TodoListViewContext, events: &mut Vec<TodoListViewEvent>) {
        // Computed values
        let current_editing_id = self.edit_state.as_ref().map(|s| s.id);

        // Commands
        let mut to_be_focused: Option<egui::Response> = None;

        // TODO: https://github.com/lucasmerlin/egui_dnd

        // Interaction
        for (id, item) in ctx.model().items.iter() {
            let id = *id;
            // NOTE: 루프 안에서는 다른 요소들이 그려지는 데 부작용을 일으킬 수 있는 작업을 피해야 한다
            // 그렇지 않으면, UI가 순간적으로 뒤바뀌거나 깜빡이는 현상이 나타날 수 있음
            // - 모든 UI 가 그려지고 난 다음에 mutation 이 이루어져야 하므로,
            //   command 를 남겨서 나중에 따로 mutation 을 할 수 있게 설계한다.
            // - 다른 요소들에 대한 mutation 을 실수로 하는 것을 막기 위해,
            //   위처럼 상태에 대한 exclusive reference 를 걸어두는 것도 좋은 방법.

            let mut completed = item.completed;

            let (
                checkbox,
                text_widget,
            ) = ui.horizontal(|ui| {
                (
                    ui.checkbox(&mut completed, ""),
                    match current_editing_id {
                        Some(i) if i == id => {
                            let edit_state = self.edit_state.as_mut().unwrap();
                            ui.text_edit_singleline(&mut edit_state.title)
                        }
                        _ => ui.add(egui::widgets::Label::new(&item.title).wrap(true)).context_menu(|ui| {
                            if ui.button("Edit").clicked() {
                                events.push(TodoListViewEvent::EditingStartedTodoItemTitle { id });
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                events.push(TodoListViewEvent::TodoItemDeleted { id });
                                ui.close_menu();
                            }
                        })
                    }
                )
            }).inner;

            let text_res = text_widget.interact(egui::Sense::click());

            // Command
            let is_editing = current_editing_id.map(|i| i == id).unwrap_or(false);
            let non_editing_item_clicked = !is_editing && text_res.clicked();
            let editing_item_enter_pressed = is_editing && Self::enter_pressed(&text_res, ui.ctx());
            let clicked_elsewhere_in_editing = is_editing && text_res.clicked_elsewhere();

            if checkbox.changed() {
                events.push(TodoListViewEvent::TodoItemToggled { id });
            }

            if non_editing_item_clicked {
                events.push(TodoListViewEvent::EditingStartedTodoItemTitle { id });
            } else if editing_item_enter_pressed || clicked_elsewhere_in_editing {
                events.push(TodoListViewEvent::EditingFinishedTodoItemTitle);
            }

            if is_editing && self.edit_state.as_ref().map(|s| s.request_focus).unwrap_or(false) {
                to_be_focused = Some(text_res.clone());
            }
        }

        // Mutation
        if let (Some(res), Some(edit_state)) = (to_be_focused, self.edit_state.as_mut()) {
            res.request_focus();
            edit_state.request_focus = false;
        }
    }

    fn try_finish_editing(&mut self, ctx: &mut TodoListViewContext) {
        let Some(EditingItem { id: item_id, title: text_for_edit, .. }) = self.edit_state.take() else { return; };
        if ctx.model().items[&item_id].title == text_for_edit {
            return;
        }
        ctx.push_command(TodoListModelCommand::UpdateTitleOfTodoItem {
            id: item_id,
            title: text_for_edit,
        });
    }

    fn enter_pressed(res: &egui::Response, egui_ctx: &egui::Context) -> bool {
        res.lost_focus() && egui_ctx.input().key_pressed(egui::Key::Enter)
    }
}
