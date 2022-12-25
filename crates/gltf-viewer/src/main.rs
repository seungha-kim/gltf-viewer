#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::Arc;
use std::time::Duration;
use gltf_engine::{AbstractKey, InputEvent, wgpu};
use gltf_engine::winit;
use gltf_engine::Engine;

use eframe::egui;
use eframe::egui::{Pos2, Vec2};

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

struct MyApp {
    prev_pointer_pos: Option<Pos2>,
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
            prev_pointer_pos: None,
        })
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let any_key_pressing = !ctx.input().keys_down.is_empty();
        let mut request_repaint = any_key_pressing;

        if ctx.input().keys_down.contains(&egui::Key::Escape) {
            frame.close();
        };
        {
            // 조금 더 추상화가 필요하겠네..
            for e in &ctx.input().events {
                log::info!("MyApp event: {:?}", e);
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
                    egui::Event::Scroll(vec) => {
                        InputEvent::MouseWheel { delta_x: vec.x, delta_y: vec.y }
                    }
                    egui::Event::PointerButton {
                        button, pressed, ..
                    } => {
                        if button == &egui::PointerButton::Primary {
                            if *pressed {
                                InputEvent::MouseLeftDown
                            } else {
                                InputEvent::MouseLeftUp
                            }
                        } else {
                            continue;
                        }
                    }
                    egui::Event::PointerMoved(vec) => {
                        if let Some(prev_pointer_pos) = self.prev_pointer_pos {
                            let mut delta = *vec - prev_pointer_pos;
                            delta *= 0.1;
                            self.prev_pointer_pos = Some(*vec);
                            InputEvent::MouseMove { delta_x: delta.x, delta_y: delta.y }
                        } else {
                            self.prev_pointer_pos = Some(*vec);
                            continue
                        }
                    }
                    _ => continue,
                };
                {
                    let wgpu_render_state = frame.wgpu_render_state().unwrap();
                    let mut write_lock = wgpu_render_state
                        .renderer
                        .write();
                    let resource = write_lock
                        .paint_callback_resources
                        .get_mut::<PaintResource>()
                        .unwrap();
                    resource.engine.input(&input_event);
                }
                request_repaint = true;
            }
        }

        // ctx.request_repaint 가 write lock 을 필요로 하기 때문에,
        // 다른 lock 이 걸려있지 않은 순간에 호출해야 함
        if request_repaint {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("glTF Viewer - ");
                        ui.hyperlink_to("Link", "https://github.com/seungha-kim/gltf-viewer");
                    });

                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        self.custom_painting(ui);
                    });
                });
        });
    }
}

impl MyApp {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
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
    }
}
