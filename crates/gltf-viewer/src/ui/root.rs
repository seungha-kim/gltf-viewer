use crate::command::{EngineCommand, EngineModel, TodoListCommand};
use crate::model::TodoListModel;
use crate::ui::framework::*;
use crate::ui::node_property::{NodePropertyViewContext, NodePropertyViewState};
use crate::ui::todo_list::{TodoListContext, TodoListViewState};
use crate::undo_manager::UndoManager;
use crate::PaintResource;
use eframe::egui;
use gltf_engine::{AbstractKey, InputEvent};
use std::sync::Arc;
use uuid::Uuid;

pub enum WorkspaceKind {
    Layout,
    TodoList(TodoListViewState),
    HelloWorld,
}

pub struct RootViewState {
    workspace: WorkspaceKind,
    node_selection: NodeSelection,
    undo_manager: UndoManager,
    todo_list: TodoListModel,
    events: Vec<RootViewEvent>,
    engine_commands: Vec<EngineCommand>,
}

impl RootViewState {
    pub fn new() -> RootViewState {
        Self {
            workspace: WorkspaceKind::Layout,
            node_selection: NodeSelection::None,
            undo_manager: UndoManager::new(),
            todo_list: TodoListModel::default(),
            events: Vec::new(),
            engine_commands: Vec::new(),
        }
    }
}

pub trait RootViewContext: ViewContext<(), EngineCommand> {
    fn engine_model(&self) -> &EngineModel;
    fn request_repaint(&mut self);
}

pub enum RootViewEvent {
    InputEvent(InputEvent),
    ChangeWorkspace(WorkspaceKind),
    ExitRequested,
    SingleNodeSelected(Uuid),
}

impl<C: RootViewContext> ViewState<(), C> for RootViewState {
    type Command = EngineCommand;

    fn interact(&mut self, ui: &mut egui::Ui, ctx: &C) {
        assert_eq!(self.engine_commands.len(), 0);
        assert_eq!(self.events.len(), 0);

        ui.ctx().set_visuals(egui::Visuals::light());
        if !ui.ctx().input().keys_down.is_empty() {
            ui.ctx().request_repaint();
        }

        if ui.ctx().input().keys_down.contains(&egui::Key::Escape) {
            self.events.push(RootViewEvent::ExitRequested);
        };

        for e in &ui.ctx().input().events {
            log::debug!("MyApp event: {:?}", e);
            let input_event = match e {
                egui::Event::Key { key, pressed, .. } => {
                    let abstract_key = match key {
                        egui::Key::ArrowUp | egui::Key::W => AbstractKey::CameraMoveForward,
                        egui::Key::ArrowDown | egui::Key::S => AbstractKey::CameraMoveBackward,
                        egui::Key::ArrowLeft | egui::Key::A => AbstractKey::CameraMoveLeft,
                        egui::Key::ArrowRight | egui::Key::D => AbstractKey::CameraMoveRight,
                        egui::Key::Q => AbstractKey::CameraMoveDown,
                        egui::Key::E => AbstractKey::CameraMoveUp,
                        _ => continue,
                    };
                    if *pressed {
                        InputEvent::KeyPressing(abstract_key)
                    } else {
                        InputEvent::KeyUp(abstract_key)
                    }
                }
                egui::Event::PointerButton {
                    button, pressed, ..
                } => {
                    // NOTE: egui::Response::drag_released 로 처리하면,
                    // 포인터가 창 밖으로 벗어난 채로 버튼을 떼었을 때 이벤트가 발생하지 않는 문제가 있어서
                    // 해당 로직만 egui::Event::PointerButton 으로 처리함 (macOS 에서 테스트됨)
                    if button == &egui::PointerButton::Secondary && !*pressed {
                        InputEvent::MouseRightUp
                    } else {
                        continue;
                    }
                }
                egui::Event::Scroll(vec) => InputEvent::MouseWheel {
                    delta_x: vec.x,
                    delta_y: vec.y,
                },
                _ => continue,
            };
            {
                self.events.push(RootViewEvent::InputEvent(input_event));
            }
        }

        self.top_panel(ui, ctx);
        self.bottom_panel(ui, ctx);
        self.left_panel(ui, ctx);
        self.right_panel(ui, ctx);
        self.central_panel(ui, ctx);
    }

    fn mutate(&mut self, ctx: &mut C) {
        for c in std::mem::take(&mut self.engine_commands) {
            ctx.push_command(c);
        }
        for e in std::mem::take(&mut self.events) {
            self.handle_event(ctx, e);
        }
    }
}

impl RootViewState {
    fn top_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, _ctx: &C) {
        let mut is_layout = false;
        let mut is_todo_list = false;
        let mut is_hello_world = false;
        match &self.workspace {
            WorkspaceKind::Layout => is_layout = true,
            WorkspaceKind::TodoList(_) => is_todo_list = true,
            WorkspaceKind::HelloWorld => is_hello_world = true,
        }
        egui::TopBottomPanel::top("my_panel").show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(is_layout, "Layout").clicked() && !is_layout {
                    self.events
                        .push(RootViewEvent::ChangeWorkspace(WorkspaceKind::Layout));
                }
                if ui.selectable_label(is_todo_list, "TodoList").clicked() && !is_todo_list {
                    self.events
                        .push(RootViewEvent::ChangeWorkspace(WorkspaceKind::TodoList(
                            TodoListViewState::new(),
                        )));
                }
                if ui.selectable_label(is_hello_world, "Hello World").clicked() && !is_hello_world {
                    self.events
                        .push(RootViewEvent::ChangeWorkspace(WorkspaceKind::HelloWorld));
                }
            });
        });
    }

    fn bottom_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, _ctx: &C) {
        egui::TopBottomPanel::bottom("my_bottom_panel").show(ui.ctx(), |ui| {
            ui.label("Hello World!");
        });
    }

    fn left_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, ctx: &C) {
        egui::SidePanel::left("my_left_panel").show(ui.ctx(), |ui| {
            ui.heading("Node Tree");
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let model_root = ctx.engine_model().engine().model_root();
                    let scene = &model_root.default_scene();
                    for &node_id in scene.nodes.iter() {
                        self.rec_node(ui, ctx, node_id);
                    }
                });
        });
    }

    fn rec_node<C: RootViewContext>(&mut self, ui: &mut egui::Ui, ctx: &C, node_id: Uuid) {
        let model_root = ctx.engine_model().engine().model_root();
        let node = &model_root.nodes[&node_id];

        let id_string = format!("Node {}", node.abbreviated_id());
        let id = ui.make_persistent_id(&id_string);
        if node.children.is_empty() {
            ui.horizontal(|ui| {
                let selected = self.node_selection.is_selected(node.id);
                if ui.selectable_label(selected, &id_string).clicked() {
                    self.events.push(RootViewEvent::SingleNodeSelected(node.id));
                };
            });
        } else {
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    let selected = self.node_selection.is_selected(node.id);
                    if ui.selectable_label(selected, &id_string).clicked() {
                        self.events.push(RootViewEvent::SingleNodeSelected(node.id));
                    }
                })
                .body(|ui| {
                    for &child_id in &node.children {
                        self.rec_node(&mut *ui, ctx, child_id);
                    }
                });
        }
    }

    fn right_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, ctx: &C) {
        ui.set_min_width(200.0);
        egui::SidePanel::right("my_right_panel").show(ui.ctx(), |ui| match &self.workspace {
            WorkspaceKind::Layout => {
                self.property_panel(ui, ctx);
            }
            WorkspaceKind::TodoList(_) => {
                self.todo_list(ui, ctx);
            }
            WorkspaceKind::HelloWorld => {
                ui.label("Hello World!");
            }
        });
    }

    fn property_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, ctx: &C) {
        if let NodeSelection::SingleSelection { id, property_view } = &mut self.node_selection {
            let mut context = NodePropertyViewContextImpl {
                node_id: *id,
                model: ctx.engine_model(),
                commands: Vec::new(),
            };
            property_view.update(ui, &mut context);
            self.engine_commands.append(&mut context.commands);
        }
    }

    fn todo_list<C: RootViewContext>(&mut self, ui: &mut egui::Ui, _ctx: &C) {
        let WorkspaceKind::TodoList(ref mut view_state) = self.workspace else { return; };

        ui.set_width(200.0);

        let mut model_commands = Vec::new();

        let (undo, redo, exit) = {
            let mut ctx =
                TodoListContextImpl::new(&self.todo_list, &self.undo_manager, &mut model_commands);

            view_state.update(ui, &mut ctx);

            (
                ctx.undo_requested(),
                ctx.redo_requested(),
                ctx.exit_requested(),
            )
        };

        if undo {
            self.undo_manager.undo(&mut self.todo_list);
        }
        if redo {
            self.undo_manager.redo(&mut self.todo_list);
        }
        if exit {
            self.events.push(RootViewEvent::ExitRequested);
        }

        for c in model_commands {
            self.undo_manager
                .push_undo(self.todo_list.process_command(c));
        }
    }

    fn central_panel<C: RootViewContext>(&mut self, ui: &mut egui::Ui, _ctx: &C) {
        let f = egui::Frame {
            inner_margin: egui::style::Margin {
                left: 0.0,
                right: 0.0,
                top: 0.0,
                bottom: 0.0,
            },
            ..Default::default()
        };

        egui::CentralPanel::default().frame(f).show(ui.ctx(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, move |ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        let response = self.custom_painting(ui);
                        if response.drag_started()
                            && response.dragged_by(egui::PointerButton::Secondary)
                        {
                            self.events
                                .push(RootViewEvent::InputEvent(InputEvent::MouseRightDown));
                            ui.output().cursor_icon = egui::CursorIcon::Move;
                        }
                        // NOTE: egui::Response::drag_released 로 처리하면,
                        // 포인터가 창 밖으로 벗어난 채로 버튼을 떼었을 때 이벤트가 발생하지 않는 문제가 있어서
                        // 해당 로직만 egui::Event::PointerButton 으로 처리함 (macOS 에서 테스트됨)
                        // if response.drag_released() {
                        //     self.engine.input(&InputEvent::MouseRightUp);
                        // }
                        if response.dragged() && response.dragged_by(egui::PointerButton::Secondary)
                        {
                            let delta = response.drag_delta() / 2.0; // FIXME: device pixel ratio?
                            self.events
                                .push(RootViewEvent::InputEvent(InputEvent::MouseMove {
                                    delta_x: delta.x,
                                    delta_y: delta.y,
                                }));
                            ui.output().cursor_icon = egui::CursorIcon::Move;
                        }
                    });
                });
        });
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let available = ui.available_rect_before_wrap();
        // TODO: scale factor
        let (rect, response) = ui.allocate_at_least(
            egui::Vec2::new(available.width(), available.height()),
            egui::Sense::drag(),
        );

        let cb = egui_wgpu::CallbackFn::new()
            .prepare(move |device, queue, _encoder, resource| {
                let resource: &mut PaintResource = resource.get_mut().unwrap();

                let physical_size = rect.size() * rect.aspect_ratio();
                let changed =
                    resource
                        .engine
                        .resize(physical_size.x as u32, physical_size.y as u32, device);
                if changed {
                    resource.update_bind_group(device);
                }
                resource.engine.update(queue);
                // TODO: parallelize
                let command_buffer = resource.engine.render(device).expect("Failed to render");
                resource.engine.end_frame();

                vec![command_buffer]
            })
            .paint(move |_info, render_pass, resource| {
                let resource: &PaintResource = resource.get().unwrap();
                resource.paint(render_pass);
            });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };

        ui.painter().add(callback);

        response
    }

    fn handle_event<C: RootViewContext>(&mut self, ctx: &mut C, event: RootViewEvent) {
        match event {
            RootViewEvent::InputEvent(input_event) => {
                ctx.push_command(EngineCommand::InputEvent(input_event));
            }
            RootViewEvent::ChangeWorkspace(workspace) => {
                self.workspace = workspace;
            }
            RootViewEvent::ExitRequested => {
                ctx.request_exit();
            }
            RootViewEvent::SingleNodeSelected(node_id) => {
                self.node_selection = NodeSelection::SingleSelection {
                    id: node_id,
                    property_view: NodePropertyViewState::new(),
                };
            }
        }
    }
}

pub struct TodoListContextImpl<'a> {
    model: &'a TodoListModel,
    undo_manager: &'a UndoManager,
    model_commands: &'a mut Vec<TodoListCommand>,
    undo_requested: bool,
    redo_requested: bool,
    exit_requested: bool,
}

impl<'a> TodoListContextImpl<'a> {
    pub fn new(
        model: &'a TodoListModel,
        undo_manager: &'a UndoManager,
        model_commands: &'a mut Vec<TodoListCommand>,
    ) -> Self {
        Self {
            model,
            undo_manager,
            model_commands,
            undo_requested: false,
            redo_requested: false,
            exit_requested: false,
        }
    }
}

impl ViewContext<TodoListModel, TodoListCommand> for TodoListContextImpl<'_> {
    fn model(&self) -> &TodoListModel {
        self.model
    }

    fn push_command(&mut self, command: TodoListCommand) {
        self.model_commands.push(command);
    }

    fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }
}

impl UndoableViewContext for TodoListContextImpl<'_> {
    fn can_undo(&self) -> bool {
        self.undo_manager.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.undo_manager.can_redo()
    }

    fn undo_requested(&self) -> bool {
        self.undo_requested
    }

    fn redo_requested(&self) -> bool {
        self.redo_requested
    }

    fn request_undo(&mut self) {
        self.undo_requested = true;
    }

    fn request_redo(&mut self) {
        self.redo_requested = true;
    }
}

impl<'a> TodoListContext for TodoListContextImpl<'a> {}

struct NodePropertyViewContextImpl<'a> {
    node_id: Uuid,
    model: &'a EngineModel<'a>,
    commands: Vec<EngineCommand>,
}

impl<'a> ViewContext<EngineModel<'a>, EngineCommand> for NodePropertyViewContextImpl<'a> {
    fn model(&self) -> &EngineModel<'a> {
        &self.model
    }

    fn push_command(&mut self, command: EngineCommand) {
        self.commands.push(command)
    }

    fn exit_requested(&self) -> bool {
        todo!()
    }

    fn request_exit(&mut self) {
        todo!()
    }
}

impl<'a> NodePropertyViewContext<'a> for NodePropertyViewContextImpl<'a> {
    fn node_id(&self) -> Uuid {
        self.node_id
    }
}

enum NodeSelection {
    None,
    SingleSelection {
        id: Uuid,
        property_view: NodePropertyViewState,
    },
}

impl NodeSelection {
    fn is_selected(&self, node_id: Uuid) -> bool {
        if let NodeSelection::SingleSelection { id, .. } = self {
            *id == node_id
        } else {
            false
        }
    }
}
