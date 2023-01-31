use eframe::egui;
use eframe::egui::{Key, Response, Sense};

pub struct TodoItem {
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
    item_index: usize,
    text_for_edit: String,
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
        self.ui.heading("To-do List");
        self.text_edit();
        self.todo_list();
    }

    fn text_edit(&mut self) {
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

        if self.enter_pressed(&text_edit) {
            self.commit_new_item();
            text_edit.request_focus();
        }

        if add_button.clicked() {
            self.commit_new_item();
            text_edit.request_focus();
        }
    }

    fn todo_list(&mut self) {
        // Props
        let editing_index = self.view_state.edit_state.as_ref().map(|s| s.item_index);

        for index in 0..self.todo_list.items.len() {
            // Render
            let (
                _checkbox,
                text_widget,
            ) = self.ui.horizontal(|ui| {
                let item = &mut self.todo_list.items[index];
                (
                    ui.checkbox(&mut item.completed, ""),
                    if editing_index.map(|i| i == index).unwrap_or(false) {
                        {
                            let edit_state = self.view_state.edit_state.as_mut().unwrap();
                            ui.text_edit_singleline(&mut edit_state.text_for_edit)
                        }
                    } else {
                        ui.add(egui::widgets::Label::new(&item.title).wrap(true))
                    },
                )
            }).inner;

            // Mutate
            let text_res = text_widget.interact(Sense::click());

            self.request_focus_if_needed(index, &text_res);

            let non_editing_item_clicked = editing_index.map(|i| i != index).unwrap_or(true) && text_res.clicked();
            let editing_item_enter_pressed = editing_index.map(|i| i == index).unwrap_or(false) && self.enter_pressed(&text_res);
            let clicked_elsewhere_in_editing = editing_index.is_some() && text_res.clicked_elsewhere();

            if non_editing_item_clicked {
                self.view_state.edit_state = Some(self.edit_state(index));
            } else if editing_item_enter_pressed || clicked_elsewhere_in_editing {
                self.commit_editing_item();
            }
        }
    }

    fn commit_new_item(&mut self) {
        if self.view_state.text_for_new.is_empty() {
            return;
        }
        self.todo_list.items.push(TodoItem {
            title: self.view_state.text_for_new.to_string(),
            completed: false,
        });

        self.view_state.text_for_new.clear();
    }

    fn commit_editing_item(&mut self) {
        let edit_state = self.view_state.edit_state.take().unwrap();
        self.todo_list.items[edit_state.item_index].title = edit_state.text_for_edit;
    }

    fn request_focus_if_needed(&mut self, index: usize, res: &Response) {
        if let Some(edit_state) = self.view_state.edit_state.as_mut() {
            if edit_state.item_index == index && edit_state.request_focus {
                res.request_focus();
                edit_state.request_focus = false;
            }
        }
    }

    fn edit_state(&self, item_index: usize) -> EditState {
        EditState {
            request_focus: true,
            item_index,
            text_for_edit: self.todo_list.items[item_index].title.clone(),
        }
    }

    fn enter_pressed(&self, res: &Response) -> bool {
        res.lost_focus() && self.egui_ctx.input().key_pressed(Key::Enter)
    }
}
