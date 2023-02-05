#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod ui;
mod model;
mod command;
mod undo_manager;

use std::sync::Arc;
use eframe::egui::style::Margin;
use gltf_engine::{AbstractKey, InputEvent, wgpu};
use gltf_engine::Engine;

use eframe::egui;
use crate::model::TodoListModel;
use crate::ui::todo_list::{TodoListViewContext, TodoListViewState};
use crate::undo_manager::UndoManager;
use crate::ui::framework::*;

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024.0, 768.0)),
        ..Default::default()
    };
    eframe::run_native(
        "glTF Viewer",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc).unwrap())),
    )
}

struct PaintResource {
    engine: Engine,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
}

impl PaintResource {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("viewport bind group layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewport pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let renderer = pollster::block_on(async {
            Engine::new(
                device,
                queue,
                100, 100, target_format,
            ).await
        });

        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );

        Self {
            engine: renderer,
            pipeline,
            bind_group_layout,
            sampler,
            bind_group: None,
        }
    }

    fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
        render_pass.draw(0..6, 0..1);
    }

    fn update_bind_group(&mut self, device: &wgpu::Device) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(self.engine.color_texture_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                }
            ],
        });
        self.bind_group = Some(bind_group);
    }
}

enum AppMode {
    TodoList(TodoListViewState),
    HelloWorld,
}

struct MyApp {
    mode: AppMode,
    undo_manager: UndoManager,
    model: TodoListModel,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;
        let device = &wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;
        let target_format = wgpu_render_state.target_format;

        let paint_resource = PaintResource::new(&device, &queue, target_format);

        wgpu_render_state
            .renderer
            .write()
            .paint_callback_resources
            .insert(paint_resource);

        Some(MyApp {
            mode: AppMode::TodoList(TodoListViewState::new()),
            undo_manager: UndoManager::new(),
            model: Default::default(),
        })
    }
}

struct MyAppContext<'a> {
    egui_ctx: &'a egui::Context,
    app: &'a mut MyApp,
    engine: &'a mut Engine,
    should_close: bool,
    request_repaint: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        let (should_close, request_repaint) = {
            let mut write_lock = frame.wgpu_render_state().unwrap().renderer.write();
            let paint_resource = write_lock.paint_callback_resources.get_mut::<PaintResource>().unwrap();
            let engine = &mut paint_resource.engine;

            let mut my_app_ctx = MyAppContext {
                egui_ctx: ctx,
                app: self,
                engine,
                should_close: false,
                request_repaint: false,
            };

            my_app_ctx.update();

            (my_app_ctx.should_close, my_app_ctx.request_repaint)
        };

        if should_close {
            frame.close();
        }

        if request_repaint {
            ctx.request_repaint();
        }
    }
}

impl<'a> MyAppContext<'a> {
    fn update(&mut self) {
        self.setup();
        self.handle_input_events();
        self.top_panel();
        self.bottom_panel();
        self.left_panel();
        self.right_panel();
        self.central_panel();
    }

    fn setup(&mut self) {
        self.egui_ctx.set_visuals(egui::Visuals::light());
        if !self.egui_ctx.input().keys_down.is_empty() {
            self.egui_ctx.request_repaint();
        }

        if self.egui_ctx.input().keys_down.contains(&egui::Key::Escape) {
            self.should_close = true;
        };
    }

    fn handle_input_events(&mut self) {
        for e in &self.egui_ctx.input().events {
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
                egui::Event::Scroll(vec) => {
                    InputEvent::MouseWheel { delta_x: vec.x, delta_y: vec.y }
                }
                _ => continue,
            };
            {
                self.engine.input(&input_event);
            }
        }
    }

    fn top_panel(&mut self) {
        let mut is_todo_list = false;
        let mut is_hello_world = false;
        match &self.app.mode {
            AppMode::TodoList(_) => is_todo_list = true,
            AppMode::HelloWorld => is_hello_world = true,
        }
        egui::TopBottomPanel::top("my_panel").show(self.egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(is_todo_list, "TodoList").clicked() && !is_todo_list {
                    self.app.mode = AppMode::TodoList(TodoListViewState::new());
                }
                if ui.selectable_label(is_hello_world, "Hello World").clicked() && !is_hello_world {
                    self.app.mode = AppMode::HelloWorld;
                }
            });
        });
    }

    fn bottom_panel(&mut self) {
        egui::TopBottomPanel::bottom("my_bottom_panel").show(self.egui_ctx, |ui| {
            ui.label("Hello World!");
        });
    }

    fn left_panel(&mut self) {
        egui::SidePanel::left("my_left_panel").show(self.egui_ctx, |ui| {
            ui.label("Hello World!");
        });
    }

    fn right_panel(&mut self) {
        egui::SidePanel::right("my_right_panel").show(self.egui_ctx, |ui| {
            match &self.app.mode {
                AppMode::TodoList(_) => self.todo_list(ui),
                AppMode::HelloWorld => {
                    ui.label("Hello World!");
                }
            }
        });
    }

    fn todo_list(&mut self, ui: &mut egui::Ui) {
        let AppMode::TodoList(ref mut view_state) = self.app.mode else {
            return;
        };
        ui.set_width(200.0);

        let mut model_commands = Vec::new();

        let (undo, redo, exit) = {
            let mut ctx = TodoListViewContext::new(
                &self.app.model,
                &self.app.undo_manager,
                &mut model_commands,
            );

            view_state.update(ui, &mut ctx);

            (ctx.undo_requested(), ctx.redo_requested(), ctx.exit_requested())
        };

        if undo {
            self.app.undo_manager.undo(&mut self.app.model);
        }
        if redo {
            self.app.undo_manager.redo(&mut self.app.model);
        }
        if exit {
            self.should_close = true;
        }

        for c in model_commands {
            self.app.undo_manager.push_undo(c.mutate(&mut self.app.model));
        }
    }

    fn central_panel(&mut self) {
        let f = egui::Frame {
            inner_margin: Margin {
                left: 0.0,
                right: 0.0,
                top: 0.0,
                bottom: 0.0,
            },
            ..Default::default()
        };

        egui::CentralPanel::default().frame(f).show(self.egui_ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, move |ui| {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        let response = self.custom_painting(ui);
                        self.handle_central_panel_drag(ui, response);
                    });
                });
        });
    }

    fn handle_central_panel_drag(&mut self, ui: &mut egui::Ui, response: egui::Response) {
        if response.drag_started() && response.dragged_by(egui::PointerButton::Secondary) {
            self.engine.input(&InputEvent::MouseRightDown);
            ui.output().cursor_icon = egui::CursorIcon::Move;
        }
        // NOTE: egui::Response::drag_released 로 처리하면,
        // 포인터가 창 밖으로 벗어난 채로 버튼을 떼었을 때 이벤트가 발생하지 않는 문제가 있어서
        // 해당 로직만 egui::Event::PointerButton 으로 처리함 (macOS 에서 테스트됨)
        // if response.drag_released() {
        //     self.engine.input(&InputEvent::MouseRightUp);
        // }
        if response.dragged() && response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta() / 2.0; // FIXME: device pixel ratio?
            self.engine.input(&InputEvent::MouseMove { delta_x: delta.x, delta_y: delta.y });
            ui.output().cursor_icon = egui::CursorIcon::Move;
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let available = ui.available_rect_before_wrap();
        // TODO: scale factor
        let (rect, response) =
            ui.allocate_at_least(egui::Vec2::new(available.width(), available.height()), egui::Sense::drag());

        let cb = egui_wgpu::CallbackFn::new()
            .prepare(move |device, queue, _encoder, resource| {
                let resource: &mut PaintResource = resource.get_mut().unwrap();

                let physical_size = rect.size() * rect.aspect_ratio();
                let changed = resource.engine.resize(physical_size.x as u32, physical_size.y as u32, device);
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
}
