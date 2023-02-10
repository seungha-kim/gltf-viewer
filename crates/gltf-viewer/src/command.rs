/*
이 파일은 다음 내용을 포함한다.

- View 가 model 을 조작하기 위해 바깥으로 전달할 command 의 정의
- Command 를 가지고 model 을 실제로 조작하는 mutate 로직
  이 로직은 command 을 원상복구하는 새로운 command 를 반환한다.

이는 다음과 같은 설계 원칙 하에 작성됐다.

- View 는 그 자체로 model 에 대한 조작을 하지 않는다. 즉, view 는 model 에 대한 immutable reference 만 제공받는다.
- View 는 model 를 간접적으로 조작하기 위해 바깥에 command 목록을 전달한다.
- Command list 를 받아서 바깥에서 어떻게 할 것인지는 view 를 사용하는 로직에서 결정한다.
- Pipeline 형태의 GUI (View stage 와 mutation stage 가 명확히 나눠짐)

이런 원칙으로 달성하고자 한 이점은 다음과 같다.

- Undo system 구현의 단순화
- View 의 재사용성 높이기
- Mutation 에 대한 통제권 확보 (순서를 조작한다던가, 일부 command 는 일부러 누락시킨다던가, ...)
 */

use gltf_engine::{Engine, InputEvent};
use uuid::Uuid;

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum TodoListCommand {
    CreateTodoItem {
        id: Option<Uuid>,
        title: String,
        completed: bool,
    },
    UpdateCompletedOfTodoItem {
        id: Uuid,
        completed: bool,
    },
    UpdateTitleOfTodoItem {
        id: Uuid,
        title: String,
    },
    DeleteTodoItem {
        id: Uuid,
    },
}

// TODO: und
#[derive(Clone, Debug)]
pub struct UpdateFloatCommand {
    pub node_id: Uuid,
    pub value: f32,
    // commit: bool,
}

#[derive(Clone, Debug)]
pub enum EngineCommand {
    InputEvent(InputEvent),
    UpdatePositionX(UpdateFloatCommand),
    UpdatePositionY(UpdateFloatCommand),
    UpdatePositionZ(UpdateFloatCommand),
    UpdateScaleX(UpdateFloatCommand),
    UpdateScaleY(UpdateFloatCommand),
    UpdateScaleZ(UpdateFloatCommand),
}

pub struct EngineModel<'a> {
    pub engine: &'a mut Engine,
}

impl<'a> EngineModel<'a> {
    pub fn new(engine: &'a mut Engine) -> Self {
        Self { engine }
    }

    pub fn engine(&self) -> &Engine {
        self.engine
    }

    pub fn process_command(&mut self, command: EngineCommand) {
        // TODO: undo
        use EngineCommand::*;
        match command {
            InputEvent(input_event) => {
                self.engine.input(&input_event);
            }
            UpdatePositionX(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.position.x = f.value;
            }
            UpdatePositionY(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.position.y = f.value;
            }
            UpdatePositionZ(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.position.z = f.value;
            }
            UpdateScaleX(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.scale.x = f.value;
            }
            UpdateScaleY(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.scale.y = f.value;
            }
            UpdateScaleZ(f) => {
                let node = self
                    .engine
                    .model_root_mut()
                    .nodes
                    .get_mut(&f.node_id)
                    .unwrap();
                node.transform.scale.z = f.value;
            }
        }
    }
}
