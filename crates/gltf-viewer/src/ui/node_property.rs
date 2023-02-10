use crate::command::{EngineCommand, EngineModel, UpdateFloatCommand};
use crate::ui::framework::{ViewContext, ViewState};
use eframe::egui;
use eframe::egui::Ui;
use uuid::Uuid;

pub enum Event {
    PositionXChanged(f32),
    PositionYChanged(f32),
    PositionZChanged(f32),
    ScaleXChanged(f32),
    ScaleYChanged(f32),
    ScaleZChanged(f32),
}

pub trait NodePropertyViewContext<'a>: ViewContext<EngineModel<'a>, EngineCommand> {
    fn node_id(&self) -> Uuid;
}

pub struct NodePropertyViewState {
    events: Vec<Event>,
}

impl NodePropertyViewState {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl<'a, C: NodePropertyViewContext<'a>> ViewState<EngineModel<'a>, C> for NodePropertyViewState {
    type Command = EngineCommand;

    fn interact(&mut self, ui: &mut Ui, ctx: &C) {
        use egui::widgets::DragValue;

        let node = ctx
            .model()
            .engine
            .model_root()
            .nodes
            .get(&ctx.node_id())
            .unwrap();
        ui.label(format!("Node {}", node.abbreviated_id()));
        ui.label(format!("Children: {}", node.children.len()));
        ui.separator();
        ui.label("Position");
        ui.horizontal(|ui| {
            let mut x = node.transform.position.x;
            let mut y = node.transform.position.y;
            let mut z = node.transform.position.z;
            if ui.add(DragValue::new(&mut x).speed(0.01)).changed() {
                self.events.push(Event::PositionXChanged(x));
            }
            if ui.add(DragValue::new(&mut y).speed(0.01)).changed() {
                self.events.push(Event::PositionYChanged(y));
            }
            if ui.add(DragValue::new(&mut z).speed(0.01)).changed() {
                self.events.push(Event::PositionZChanged(z));
            }
        });
        ui.separator();
        ui.label("Rotation (TODO)");
        ui.separator();
        ui.label("Scale");
        ui.horizontal(|ui| {
            let mut x = node.transform.scale.x;
            let mut y = node.transform.scale.y;
            let mut z = node.transform.scale.z;
            if ui.add(DragValue::new(&mut x).speed(0.01)).changed() {
                self.events.push(Event::ScaleXChanged(x));
            };
            if ui.add(DragValue::new(&mut y).speed(0.01)).changed() {
                self.events.push(Event::ScaleYChanged(y));
            };
            if ui.add(DragValue::new(&mut z).speed(0.01)).changed() {
                self.events.push(Event::ScaleZChanged(z));
            }
        });
    }

    fn mutate(&mut self, ctx: &mut C) {
        for e in std::mem::take(&mut self.events) {
            self.handle_event(ctx, e);
        }
    }
}

impl NodePropertyViewState {
    fn handle_event<'a, C: NodePropertyViewContext<'a>>(&mut self, ctx: &mut C, event: Event) {
        let node_id = ctx.node_id();
        match event {
            Event::PositionXChanged(value) => {
                ctx.push_command(EngineCommand::UpdatePositionX(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
            Event::PositionYChanged(value) => {
                ctx.push_command(EngineCommand::UpdatePositionY(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
            Event::PositionZChanged(value) => {
                ctx.push_command(EngineCommand::UpdatePositionZ(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
            Event::ScaleXChanged(value) => {
                ctx.push_command(EngineCommand::UpdateScaleX(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
            Event::ScaleYChanged(value) => {
                ctx.push_command(EngineCommand::UpdateScaleY(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
            Event::ScaleZChanged(value) => {
                ctx.push_command(EngineCommand::UpdateScaleZ(UpdateFloatCommand {
                    node_id,
                    value,
                }))
            }
        }
    }
}
