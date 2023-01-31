use eframe::egui;
use eframe::egui::{Key, Response};

pub struct TodoItem {
    title: String,
    completed: bool,
}

pub struct TodoListViewState {
    text_input: String,
}

impl TodoListViewState {
    pub fn new() -> Self {
        Self {
            text_input: "".into(),
        }
    }
}

pub struct TodoList {
    items: Vec<TodoItem>,
}

impl TodoList {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}

pub struct TodoListContext<'a> {
    pub egui_ctx: &'a egui::Context,
    pub view_state: &'a mut TodoListViewState,
    pub todo_list: &'a mut TodoList,
    pub ui: &'a mut egui::Ui,
}

impl<'a> TodoListContext<'a> {
    pub fn update(&mut self) {
        self.text_edit();
        self.todo_list();
    }

    fn text_edit(&mut self) {
        let (text_edit, add_button) = self
            .ui
            .horizontal(|ui| {
                (
                    ui.text_edit_singleline(&mut self.view_state.text_input),
                    ui.button("Add"),
                )
            })
            .inner;

        if text_edit.lost_focus() && self.egui_ctx.input().key_pressed(Key::Enter) {
            self.add_todo_item();
            text_edit.request_focus();
        }

        if add_button.clicked() {
            self.add_todo_item();
            text_edit.request_focus();
        }
    }

    fn todo_list(&mut self) {
        for item in &mut self.todo_list.items {
            self.ui.horizontal(|ui| {
                ui.checkbox(&mut item.completed, "");
                ui.label(&item.title);
            });
        }
    }

    fn add_todo_item(&mut self) {
        self.todo_list.items.push(TodoItem {
            title: self.view_state.text_input.to_string(),
            completed: false,
        });

        self.view_state.text_input.clear();
    }
}
