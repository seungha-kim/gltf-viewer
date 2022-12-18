mod texture;
mod camera;
mod model;
mod import;
mod image_util;

use wgpu::include_wgsl;
use wgpu::util::DeviceExt;
use winit::event_loop::ControlFlow;
use winit::window::Window;
use winit::{event::*, event_loop::EventLoop, window::WindowBuilder};
use cgmath::*;
use crate::camera::CameraController;

// Renderer 는 Window 나 UI 에 대해서는 몰라야 한다
// 비즈니스 로직에 대해서도 몰라야 한다. 오직 렌더링에 대해서만

// 렌더러의 책임
// 렌더링에 필요한 자원을 관리한다
// 모델의 변경사항을 받아서 자원을 업데이트한다
// 자원을 가지고 화면을 그린다

// 현재 이 파일의 문제
// 모델과 렌더러로 분리되어야 하는 요소들이 막 섞여 있음 - 차근차근 분리해보자
// 단일 primitive 밖에 그릴 수 없음 - 여러 개의 primitive 를 그릴 수 있게 바꿔보자 - 이어서 gltf

// gltf -> model -> renderer
// UI -> model update -> render

// 특이사항 - vertex buffer 의 레이아웃은 사전에 알 수 없다. gltf 파일 마다 다를 수 있다. 단 position, normal, texcoord 가 있다는 가정 정도는 해도 괜찮을듯 (정 없으면 만들어 넣으면 되니까)
// Vertex struct 를 굳이 만들 필요도 없음
// 이 때 vertex layout 이 다른 유형마다 각각 Render pipeline 을 만들어주어야 함. shader 코드는 같아도 됨

pub struct Renderer {
    // pipeline resource
    render_pipeline: wgpu::RenderPipeline,
    depth_texture: texture::Texture,

    model_root: model::ImportedGltf,

    // layout
    #[allow(dead_code)]
    camera_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    node_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    material_bind_group_layout: wgpu::BindGroupLayout,

    // camera state
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,

    // camera resource
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // UI state
    mouse_pressed: bool,

    // etc
    #[allow(dead_code)]
    white_texture: texture::Texture,
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct NodeUniform {
    model_mat: [[f32; 4]; 4],
    normal_mat: [[f32; 4]; 4],
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MaterialUniform {
    base_color_factor: [f32; 4],
    emissive_factor: [f32; 3],
    _pad: f32,
}


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_front: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: cgmath::Vector4::zero().into(),
            view_front: cgmath::Vector4::unit_x().into(),
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        let f = camera.front();
        self.view_front = Vector4::new(f.x, f.y, f.z, 0.0).into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexPosition([f32; 3]);

impl VertexPosition {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ]
        }
    }
}

struct VertexNormal([f32; 3]);

impl VertexNormal {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexTexCoord([f32; 2]);

impl VertexTexCoord {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ]
        }
    }
}

impl Renderer {
    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) -> Self { ;
        let node_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("node_bind_group_layout"),
        });

        let material_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("material_bind_group_layout"),
        });

        // TODO: main 으로 빼기
        let gltf_root = {
            let args = std::env::args().collect::<Vec<_>>();
            let (document, buffers, images) = gltf::import(&args[1]).unwrap();
            import::GltfRoot {
                document,
                buffers,
                images,
            }
        };

        let white_image = image_util::white_image();
        let white_texture = texture::Texture::from_image(&device, &queue, &white_image, Some("White")).unwrap();

        let model_root = import::import_gltf(&gltf_root, &import::WgpuDeps {
            device: &device,
            queue: &queue,
            node_uniform_layout: &node_bind_group_layout,
            material_uniform_layout: &material_bind_group_layout,
            white_texture: &white_texture,
        });

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection = camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &material_bind_group_layout,
                    &camera_bind_group_layout,
                    &node_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexPosition::desc(),
                    VertexNormal::desc(),
                    VertexTexCoord::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),

                polygon_mode: wgpu::PolygonMode::Fill,

                unclipped_depth: false,

                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            render_pipeline,
            model_root,
            camera,
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            mouse_pressed: false,
            camera_bind_group_layout,
            node_bind_group_layout,
            material_bind_group_layout,
            depth_texture,
            white_texture,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture")
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                KeyboardInput {
                    virtual_keycode: Some(key),
                    state,
                    ..
                },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: instant::Duration, queue: &wgpu::Queue) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera, &self.projection);

        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    pub fn render(&mut self, device: &wgpu::Device, surface: &wgpu::Surface, queue: &wgpu::Queue) -> Result<(), wgpu::SurfaceError> {
        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.8,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            let mut node_stack: Vec<(&model::Node, Matrix4<f32>)> = Vec::new();

            let scene = &self.model_root.scenes[self.model_root.default_scene_id];
            for root_node_index in &scene.nodes {
                node_stack.push((&self.model_root.nodes[*root_node_index], Matrix4::identity()));
            }

            while let Some((node, upper_transform)) = node_stack.pop() {
                // TODO: 매번 write_buffer 할 필요 없음
                // TODO: cgmath::Matrix4 가 bytemuck 이랑 연동되면 좋을텐데 -> nalgebra?
                let transform = upper_transform * node.transform;
                let rs = Matrix3::from_cols(transform.x.truncate(), transform.y.truncate(), transform.z.truncate());
                let node_uniform = NodeUniform {
                    model_mat: transform.into(),
                    normal_mat: Matrix4::from(rs.invert().unwrap().transpose()).into(),
                };
                queue.write_buffer(&node.uniform_buffer, 0, bytemuck::cast_slice(&[node_uniform]));

                // draw mesh
                if let Some(mesh_index) = node.mesh_index {
                    let mesh = &self.model_root.meshes[mesh_index];
                    for primitive in mesh.primitives.iter() {
                        if primitive.is_none() { continue; }
                        let primitive = primitive.as_ref().unwrap();

                        // TODO: default material
                        let material_id = if let Some(id) = primitive.material_id { id } else { continue; };
                        let material = &self.model_root.materials[material_id];

                        let model::MeshPrimitive {
                            position_buffer,
                            normal_buffer,
                            tex_coord_buffer,
                            index_buffer,
                            index_format,
                            num_indices,
                            ..
                        } = &primitive;

                        render_pass.set_bind_group(2, &node.uniform_bind_group, &[]);
                        render_pass.set_bind_group(0, &material.material_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, position_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, normal_buffer.slice(..));
                        render_pass.set_vertex_buffer(2, tex_coord_buffer.slice(..));
                        render_pass.set_index_buffer(index_buffer.slice(..), *index_format);
                        render_pass.draw_indexed(0..(*num_indices as u32), 0, 0..1);
                    }
                }

                // visit children
                for child_index in &node.children {
                    let child = &self.model_root.nodes[*child_index];
                    node_stack.push((child, transform))
                }
            }
        }
        let command_buffer = encoder.finish();

        queue.submit(std::iter::once(command_buffer));
        output.present();

        Ok(())
    }

    pub fn mouse_pressed(&self) -> bool {
        self.mouse_pressed
    }

    pub fn camera_controller_mut(&mut self) -> &mut CameraController {
        &mut self.camera_controller
    }
}
