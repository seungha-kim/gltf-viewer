use std::collections::HashMap;
use eframe::egui;
use eframe::egui::{Key, Response, Sense, TextBuffer};

pub struct TodoItem {
    id: uuid::Uuid,
    title: String,
    completed: bool,
}

pub struct TodoListViewState {
    text_for_new: String,
    edit_state: Option<EditState>,
}

impl TodoListViewState {
    pub fn new() -> Self {
        Self {
            text_for_new: "".into(),
            edit_state: None,
        }
    }
}

struct EditState {
    request_focus: bool,
    item_id: uuid::Uuid,
    text_for_edit: String,
}

pub struct TodoList {
    items: HashMap<uuid::Uuid, TodoItem>,
    undo_stack: Vec<ModelCommand>,
    redo_stack: Vec<ModelCommand>,
    // TODO: order
}

impl TodoList {
    pub fn new() -> Self {
        let items = vec![
            TodoItem {
                id: uuid::Uuid::new_v4(),
                title: "egui Basics".into(),
                completed: true,
            },
            TodoItem {
                id: uuid::Uuid::new_v4(),
                title: "egui Intermediate".into(),
                completed: false,
            },
            TodoItem {
                id: uuid::Uuid::new_v4(),
                title: "egui Complex Application".into(),
                completed: false,
            },
        ];
        let mut map = HashMap::new();
        for item in items {
            map.insert(item.id, item);
        }
        Self {
            items: map,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn handle_command(&mut self, command: ModelCommand) {
        self.redo_stack.clear();
        let reverse = self.mutate(command);
        self.undo_stack.push(reverse);
    }

    pub fn undo(&mut self) {
        let Some(command) = self.undo_stack.pop() else { return; };
        let reverse = self.mutate(command);
        self.redo_stack.push(reverse);
    }

    pub fn redo(&mut self) {
        let Some(command) = self.redo_stack.pop() else { return; };
        let reverse = self.mutate(command);
        self.undo_stack.push(reverse);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Mutate the model content by given command, and return the reverse.
    fn mutate(&mut self, command: ModelCommand) -> ModelCommand {
        match command {
            ModelCommand::CreateTodoItem { id, title, completed } => {
                let id = id.unwrap_or_else(uuid::Uuid::new_v4);
                self.items.insert(id, TodoItem {
                    id,
                    title,
                    completed,
                });
                ModelCommand::DeleteTodoItem {
                    id
                }
            }
            ModelCommand::UpdateCompletedOfTodoItem { id, completed } => {
                let item = self.items.get_mut(&id).expect("Can't find with id");
                item.completed = completed;
                ModelCommand::UpdateCompletedOfTodoItem {
                    id,
                    completed: !completed,
                }
            }
            ModelCommand::UpdateTitleOfTodoItem { id, mut title } => {
                let item = self.items.get_mut(&id).expect("Can't find with id");
                std::mem::swap(&mut item.title, &mut title);
                ModelCommand::UpdateTitleOfTodoItem {
                    id,
                    title,
                }
            }
            ModelCommand::DeleteTodoItem { id } => {
                let TodoItem { id, title, completed } = self.items.remove(&id).expect("Can't find with id");
                ModelCommand::CreateTodoItem {
                    id: Some(id),
                    title,
                    completed,
                }
            }
        }
    }
}

// Not undoable!
enum ViewEvent {
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
}

// Undoable!
#[derive(Clone, Debug)]
enum ModelCommand {
    CreateTodoItem {
        id: Option<uuid::Uuid>,
        title: String,
        completed: bool,
    },
    UpdateCompletedOfTodoItem {
        id: uuid::Uuid,
        completed: bool,
    },
    UpdateTitleOfTodoItem {
        id: uuid::Uuid,
        title: String,
    },
    DeleteTodoItem {
        id: uuid::Uuid,
    },
}

// TODO: mutable reference for model should become immutable reference
fn handle_view_event<'a>(view_event: ViewEvent, view_state: &'a mut TodoListViewState, model: &'a TodoList) -> Vec<ModelCommand> {
    let mut model_commands = Vec::new();
    'outer: {
        match view_event {
            ViewEvent::TodoItemCreated => {
                let title = view_state.text_for_new.take();
                model_commands.push(ModelCommand::CreateTodoItem {
                    id: None,
                    title,
                    completed: false,
                })
            }
            ViewEvent::EditingStartedTodoItemTitle { id } => {
                view_state.edit_state = Some(EditState {
                    item_id: id,
                    text_for_edit: model.items[&id].title.clone(),
                    request_focus: true,
                });
            }
            ViewEvent::EditingFinishedTodoItemTitle => {
                let Some(EditState { item_id, text_for_edit, .. }) = view_state.edit_state.take() else { break 'outer; };
                model_commands.push(ModelCommand::UpdateTitleOfTodoItem {
                    id: item_id,
                    title: text_for_edit,
                });
            }
            ViewEvent::TodoItemDeleted { id } => {
                if view_state.edit_state.is_some() {
                    model_commands.append(&mut handle_view_event(ViewEvent::EditingFinishedTodoItemTitle, view_state, model));
                }
                model_commands.push(ModelCommand::DeleteTodoItem { id });
            }
            ViewEvent::TodoItemToggled { id } => {
                let item = model.items.get(&id).expect("Can't find with id");
                model_commands.push(ModelCommand::UpdateCompletedOfTodoItem {
                    id,
                    completed: !item.completed,
                });
            }
        }
    }
    model_commands
}

pub struct TodoListContext<'a> {
    pub egui_ctx: &'a egui::Context,
    pub view_state: &'a mut TodoListViewState,
    pub todo_list: &'a mut TodoList,
    pub ui: &'a mut egui::Ui,
}

impl<'a> TodoListContext<'a> {
    pub fn update(&mut self) {
        // TODO: UI 는 self 에서 없애고, 파라미터로 받아야 self 에 대한 mutable 참조를 안 쓸 수 있음
        self.ui.heading("To-do List");
        if self.ui.add_enabled(self.todo_list.can_undo(), egui::widgets::Button::new("Undo")).clicked() {
            self.todo_list.undo();
        }
        if self.ui.add_enabled(self.todo_list.can_redo(), egui::widgets::Button::new("Redo")).clicked() {
            self.todo_list.redo();
        }
        {
            let input = &self.egui_ctx.input();
            // NOTE: fizz-buzz!
            if input.modifiers.command && input.modifiers.shift && input.key_pressed(Key::Z) {
                self.todo_list.redo();
            } else if input.modifiers.command && input.key_pressed(Key::Z) {
                self.todo_list.undo();
            }
        }

        let mut view_events = Vec::new();
        view_events.append(&mut self.text_edit());
        view_events.append(&mut self.todo_list());

        for vc in view_events {
            let commands = handle_view_event(vc, &mut self.view_state, &mut self.todo_list);
            for c in commands {
                self.todo_list.handle_command(c);
            }
        }
    }

    fn text_edit(&mut self) -> Vec<ViewEvent> {
        let mut view_events = Vec::new();
        // Props
        let is_empty_text = self.view_state.text_for_new.is_empty();

        let (text_edit, add_button) = self
            .ui
            .horizontal(|ui| {
                (
                    ui.text_edit_singleline(&mut self.view_state.text_for_new),
                    ui.add_enabled(!is_empty_text, egui::widgets::Button::new("Add")),
                )
            })
            .inner;

        if Self::enter_pressed(&text_edit, self.egui_ctx) || add_button.clicked() {
            text_edit.request_focus();
            view_events.push(ViewEvent::TodoItemCreated);
        }
        
        view_events
    }

    // TODO: immutable reference
    fn todo_list(&mut self) -> Vec<ViewEvent> {
        // Computed values
        let current_editing_id = self.view_state.edit_state.as_ref().map(|s| s.item_id);

        // Commands
        let mut view_events: Vec<ViewEvent> = Vec::new();
        let mut to_be_focused: Option<Response> = None;

        // TODO: https://github.com/lucasmerlin/egui_dnd

        // Interaction
        for (id, item) in self.todo_list.items.iter() {
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
            ) = self.ui.horizontal(|ui| {
                (
                    ui.checkbox(&mut completed, ""),
                    match current_editing_id {
                        Some(i) if i == id => {
                            let edit_state = self.view_state.edit_state.as_mut().unwrap();
                            ui.text_edit_singleline(&mut edit_state.text_for_edit)
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
            let editing_item_enter_pressed = is_editing && Self::enter_pressed(&text_res, &self.egui_ctx);
            let clicked_elsewhere_in_editing = is_editing && text_res.clicked_elsewhere();

            if checkbox.changed() {
                view_events.push(ViewEvent::TodoItemToggled { id });
            }

            if non_editing_item_clicked {
                view_events.push(ViewEvent::EditingStartedTodoItemTitle { id });
            } else if editing_item_enter_pressed || clicked_elsewhere_in_editing {
                view_events.push(ViewEvent::EditingFinishedTodoItemTitle);
            }

            if is_editing && self.view_state.edit_state.as_ref().map(|s| s.request_focus).unwrap_or(false) {
                to_be_focused = Some(text_res.clone());
            }
        }

        // Mutation
        if let (Some(res), Some(edit_state)) = (to_be_focused, self.view_state.edit_state.as_mut()) {
            res.request_focus();
            edit_state.request_focus = false;
        }
        
        view_events
    }

    fn request_focus_if_needed(&mut self, id: uuid::Uuid, res: &Response) {
        if let Some(edit_state) = self.view_state.edit_state.as_mut() {
            if edit_state.item_id == id && edit_state.request_focus {
                res.request_focus();
                edit_state.request_focus = false;
            }
        }
    }

    fn edit_state(&self, item_id: uuid::Uuid) -> EditState {
        EditState {
            request_focus: true,
            item_id,
            text_for_edit: self.todo_list.items[&item_id].title.clone(),
        }
    }

    fn enter_pressed(res: &Response, egui_ctx: &egui::Context) -> bool {
        res.lost_focus() && egui_ctx.input().key_pressed(Key::Enter)
    }
}
