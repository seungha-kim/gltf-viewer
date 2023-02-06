use std::collections::HashMap;
use crate::command::TodoListCommand;

pub struct TodoItem {
    pub id: uuid::Uuid,
    pub title: String,
    pub completed: bool,
}

// TODO: more conservative interface
pub struct TodoListModel {
    pub items: HashMap<uuid::Uuid, TodoItem>,
    pub item_order: Vec<uuid::Uuid>,
}

impl TodoListModel {
    pub fn process_command(&mut self, command: TodoListCommand) -> TodoListCommand {
        match command {
            TodoListCommand::CreateTodoItem { id, title, completed } => {
                let id = id.unwrap_or_else(uuid::Uuid::new_v4);
                self.items.insert(id, TodoItem {
                    id,
                    title,
                    completed,
                });
                TodoListCommand::DeleteTodoItem {
                    id
                }
            }
            TodoListCommand::UpdateCompletedOfTodoItem { id, completed } => {
                let item = self.items.get_mut(&id).expect("Can't find with id");
                item.completed = completed;
                TodoListCommand::UpdateCompletedOfTodoItem {
                    id,
                    completed: !completed,
                }
            }
            TodoListCommand::UpdateTitleOfTodoItem { id, mut title } => {
                let item = self.items.get_mut(&id).expect("Can't find with id");
                std::mem::swap(&mut item.title, &mut title);
                TodoListCommand::UpdateTitleOfTodoItem {
                    id,
                    title,
                }
            }
            TodoListCommand::DeleteTodoItem { id } => {
                let TodoItem { id, title, completed } = self.items.remove(&id).expect("Can't find with id");
                TodoListCommand::CreateTodoItem {
                    id: Some(id),
                    title,
                    completed,
                }
            }
        }
    }
}

impl From<Vec<TodoItem>> for TodoListModel {
    fn from(items: Vec<TodoItem>) -> Self {
        let mut map = HashMap::new();
        for item in items {
            map.insert(item.id, item);
        }
        TodoListModel {
            items: map,
            item_order: Vec::new(),
        }
    }
}

impl Default for TodoListModel {
    fn default() -> Self {
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
        items.into()
    }
}
