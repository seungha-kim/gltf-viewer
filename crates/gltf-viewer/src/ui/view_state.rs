use eframe::egui;
use eframe::egui::{Key, Response, Sense};
use crate::command::ModelCommand;
use crate::global_event::GlobalEvent;
use crate::model::Model;
use crate::ui::component::ComponentContext;
use crate::undo_manager::UndoManager;

pub(super) struct TodoListViewState {
    pub new_title: String,
    pub edit_state: Option<EditingItem>,
}

impl TodoListViewState {
    pub fn new() -> Self {
        Self {
            new_title: "".into(),
            edit_state: None,
        }
    }
}

pub(super) struct EditingItem {
    pub request_focus: bool,
    pub id: uuid::Uuid,
    pub title: String,
}

pub(super) enum ViewEvent {
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
    GlobalEvent(GlobalEvent),
}

impl TodoListViewState {
    pub fn update(&mut self, ui: &mut egui::Ui, ctx: &ComponentContext) -> Vec<ViewEvent> {
        let mut view_events = Vec::new();
        ui.heading("To-do List");
        if ui.add_enabled(ctx.undo_manager.can_undo(), egui::widgets::Button::new("Undo")).clicked() {
            view_events.push(ViewEvent::GlobalEvent(GlobalEvent::UndoRequested));
        }
        if ui.add_enabled(ctx.undo_manager.can_redo(), egui::widgets::Button::new("Redo")).clicked() {
            view_events.push(ViewEvent::GlobalEvent(GlobalEvent::RedoRequested));
        }

        view_events.append(&mut self.text_edit(ui, ctx));
        view_events.append(&mut self.todo_list(ui, ctx));

        let input = &ui.ctx().input();
        // NOTE: fizz-buzz!
        if input.modifiers.command && input.modifiers.shift && input.key_pressed(Key::Z) {
            view_events.push(ViewEvent::GlobalEvent(GlobalEvent::RedoRequested));
        } else if input.modifiers.command && input.key_pressed(Key::Z) {
            view_events.push(ViewEvent::GlobalEvent(GlobalEvent::UndoRequested));
        }

        view_events
    }

    fn text_edit(&mut self, ui: &mut egui::Ui, ctx: &ComponentContext) -> Vec<ViewEvent> {
        let mut view_events = Vec::new();
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
            view_events.push(ViewEvent::TodoItemCreated);
        }

        view_events
    }

    // TODO: immutable reference
    fn todo_list(&mut self, ui: &mut egui::Ui, ctx: &ComponentContext) -> Vec<ViewEvent> {
        // Computed values
        let current_editing_id = self.edit_state.as_ref().map(|s| s.id);

        // Commands
        let mut view_events: Vec<ViewEvent> = Vec::new();
        let mut to_be_focused: Option<Response> = None;

        // TODO: https://github.com/lucasmerlin/egui_dnd

        // Interaction
        for (id, item) in ctx.model.items.iter() {
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
                                view_events.push(ViewEvent::EditingStartedTodoItemTitle { id });
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                view_events.push(ViewEvent::TodoItemDeleted { id });
                                ui.close_menu();
                            }
                        })
                    }
                )
            }).inner;

            let text_res = text_widget.interact(Sense::click());

            // Command
            let is_editing = current_editing_id.map(|i| i == id).unwrap_or(false);
            let non_editing_item_clicked = !is_editing && text_res.clicked();
            let editing_item_enter_pressed = is_editing && Self::enter_pressed(&text_res, ui.ctx());
            let clicked_elsewhere_in_editing = is_editing && text_res.clicked_elsewhere();

            if checkbox.changed() {
                view_events.push(ViewEvent::TodoItemToggled { id });
            }

            if non_editing_item_clicked {
                view_events.push(ViewEvent::EditingStartedTodoItemTitle { id });
            } else if editing_item_enter_pressed || clicked_elsewhere_in_editing {
                view_events.push(ViewEvent::EditingFinishedTodoItemTitle);
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

        view_events
    }

    fn request_focus_if_needed(&mut self, id: uuid::Uuid, res: &Response) {
        if let Some(edit_state) = self.edit_state.as_mut() {
            if edit_state.id == id && edit_state.request_focus {
                res.request_focus();
                edit_state.request_focus = false;
            }
        }
    }

    fn edit_state(&self, item_id: uuid::Uuid, ctx: &ComponentContext) -> EditingItem {
        EditingItem {
            request_focus: true,
            id: item_id,
            title: ctx.model.items[&item_id].title.clone(),
        }
    }

    fn enter_pressed(res: &Response, egui_ctx: &egui::Context) -> bool {
        res.lost_focus() && egui_ctx.input().key_pressed(Key::Enter)
    }
}
