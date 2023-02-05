use std::collections::HashMap;

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