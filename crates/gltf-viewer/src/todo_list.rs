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

        if Self::enter_pressed(&text_edit, self.egui_ctx) {
            self.commit_new_item();
            text_edit.request_focus();
        }

        if add_button.clicked() {
            self.commit_new_item();
            text_edit.request_focus();
        }
    }

    fn todo_list(&mut self) {
        // Computed values
        let current_editing_index = self.view_state.edit_state.as_ref().map(|s| s.item_index);

        // Command variable
        let mut to_be_edited: Option<usize> = None;
        let mut to_be_commited = false;
        let mut to_be_focused: Option<Response> = None;
        let mut to_be_deleted: Option<usize> = None;

        // Interaction
        for (index, item) in self.todo_list.items.iter_mut().enumerate() {
            // NOTE: 루프 안에서는 다른 요소들이 그려지는 데 부작용을 일으킬 수 있는 작업을 피해야 한다
            // 그렇지 않으면, UI가 순간적으로 뒤바뀌거나 깜빡이는 현상이 나타날 수 있음
            // - 모든 UI 가 그려지고 난 다음에 mutation 이 이루어져야 하므로,
            //   command 를 남겨서 나중에 따로 mutation 을 할 수 있게 설계한다.
            // - 다른 요소들에 대한 mutation 을 실수로 하는 것을 막기 위해,
            //   위처럼 상태에 대한 exclusive reference 를 걸어두는 것도 좋은 방법.
            let (
                _checkbox,
                text_widget,
            ) = self.ui.horizontal(|ui| {
                (
                    ui.checkbox(&mut item.completed, ""),
                    match current_editing_index {
                        Some(i) if i == index => {
                            let edit_state = self.view_state.edit_state.as_mut().unwrap();
                            ui.text_edit_singleline(&mut edit_state.text_for_edit)
                        }
                        _ => ui.add(egui::widgets::Label::new(&item.title).wrap(true)).context_menu(|ui| {
                            if ui.button("Edit").clicked() {
                                to_be_edited = Some(index);
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                to_be_deleted = Some(index);
                                ui.close_menu();
                            }
                        })
                    }
                )
            }).inner;

            let text_res = text_widget.interact(Sense::click());

            // Command
            let is_editing = current_editing_index.map(|i| i == index).unwrap_or(false);
            let non_editing_item_clicked = !is_editing && text_res.clicked();
            let editing_item_enter_pressed = is_editing && Self::enter_pressed(&text_res, &self.egui_ctx);
            let clicked_elsewhere_in_editing = is_editing && text_res.clicked_elsewhere();

            if non_editing_item_clicked {
                to_be_edited = Some(index);
            } else if editing_item_enter_pressed || clicked_elsewhere_in_editing {
                to_be_commited = true;
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

        if let Some(index) = to_be_edited {
            self.view_state.edit_state = Some(self.edit_state(index));
        } else if to_be_commited {
            self.commit_editing_item();
        }

        if let Some(index) = to_be_deleted {
            self.commit_editing_item();
            self.todo_list.items.remove(index);
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
        let Some(edit_state) = self.view_state.edit_state.take() else { return; };
        self.todo_list.items[edit_state.item_index].title = edit_state.text_for_edit.clone();
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

    fn enter_pressed(res: &Response, egui_ctx: &egui::Context) -> bool {
        res.lost_focus() && egui_ctx.input().key_pressed(Key::Enter)
    }
}
