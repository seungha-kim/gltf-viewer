use crate::model::{Model, TodoItem};

// Undoable!
#[derive(Clone, Debug)]
pub enum ModelCommand {
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

impl ModelCommand {
    /// Mutate the model content by given command, and return the reverse.
    pub fn mutate(self, model: &mut Model) -> ModelCommand {
        match self {
            ModelCommand::CreateTodoItem { id, title, completed } => {
                let id = id.unwrap_or_else(uuid::Uuid::new_v4);
                model.items.insert(id, TodoItem {
                    id,
                    title,
                    completed,
                });
                ModelCommand::DeleteTodoItem {
                    id
                }
            }
            ModelCommand::UpdateCompletedOfTodoItem { id, completed } => {
                let item = model.items.get_mut(&id).expect("Can't find with id");
                item.completed = completed;
                ModelCommand::UpdateCompletedOfTodoItem {
                    id,
                    completed: !completed,
                }
            }
            ModelCommand::UpdateTitleOfTodoItem { id, mut title } => {
                let item = model.items.get_mut(&id).expect("Can't find with id");
                std::mem::swap(&mut item.title, &mut title);
                ModelCommand::UpdateTitleOfTodoItem {
                    id,
                    title,
                }
            }
            ModelCommand::DeleteTodoItem { id } => {
                let TodoItem { id, title, completed } = model.items.remove(&id).expect("Can't find with id");
                ModelCommand::CreateTodoItem {
                    id: Some(id),
                    title,
                    completed,
                }
            }
        }
    }
}
