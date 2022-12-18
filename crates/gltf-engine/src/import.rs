use crate::{MaterialUniform, model, NodeUniform};
use wgpu::util::DeviceExt;
use crate::texture;

pub struct GltfRoot {
    pub document: gltf::Document,
    pub buffers: Vec<gltf::buffer::Data>,
    pub images: Vec<gltf::image::Data>,
}

pub struct WgpuDeps<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub node_uniform_layout: &'a wgpu::BindGroupLayout,
    pub material_uniform_layout: &'a wgpu::BindGroupLayout,
    pub white_texture: &'a texture::Texture,
}

pub fn import_gltf(root: &GltfRoot, deps: &WgpuDeps) -> model::ImportedGltf {
    let document = &root.document;

    let default_scene_id = document
        .default_scene()
        .map(|scene| scene.index())
        .unwrap_or(0);

    let scenes = document
        .scenes()
        .map(import_scene)
        .collect();

    let nodes = document
        .nodes()
        .map(|n| import_node(n, deps))
        .collect();

    let meshes = document
        .meshes()
        .map(|mesh| import_mesh(mesh, root, deps))
        .collect();

    // TODO: texture, sampler

    let materials = document.materials().map(|m| import_material(m, deps)).collect();

    model::ImportedGltf {
        default_scene_id,
        scenes,
        nodes,
        meshes,
        materials,
    }
}

fn import_scene(scene: gltf::Scene) -> model::Scene {
    let gltf_index = scene.index();
    let mut nodes = Vec::new();
    for root_node in scene.nodes() {
        nodes.push(root_node.index());
    }
    model::Scene { gltf_index, nodes }
}

fn import_node(node: gltf::Node, deps: &WgpuDeps) -> model::Node {
    let transform = import_transform(node.transform());

    let uniform_buffer = deps.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Uniform Buffer"),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        size: std::mem::size_of::<NodeUniform>() as wgpu::BufferAddress,
        mapped_at_creation: false,
    });

    let uniform_bind_group = deps.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &deps.node_uniform_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }
        ],
        label: Some("primitive_transform_bind_group"),
    });

    model::Node {
        gltf_index: node.index(),
        transform,
        children: node.children().map(|child| child.index()).collect(),
        mesh_index: node.mesh().map(|m| m.index()),
        uniform_buffer,
        uniform_bind_group,
    }
}

fn import_transform(transform: gltf::scene::Transform) -> cgmath::Matrix4<f32> {
    use gltf::scene::Transform;
    match transform {
        Transform::Matrix { matrix } => matrix.into(),
        Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => {
            let translation_mat = cgmath::Matrix4::from_translation(translation.into());
            let rotation_mat: cgmath::Matrix4<f32> = cgmath::Quaternion::from(rotation).into();
            let scale_mat = cgmath::Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
            translation_mat * rotation_mat * scale_mat
        }
    }
}

fn import_material(material: gltf::Material, deps: &WgpuDeps) -> model::Material {
    if material.double_sided() {
        log::warn!("Double sided material found");
    }
    let emissive_factor: cgmath::Vector3<f32> = material.emissive_factor().into();
    let mr = material.pbr_metallic_roughness();
    let base_color_factor: cgmath::Vector4<f32> = mr.base_color_factor().into();
    let material_uniform = MaterialUniform {
        base_color_factor: base_color_factor.into(),
        emissive_factor: emissive_factor.into(),
        _pad: 0.0,
    };

    let uniform_buffer = deps.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Material Uniform Buffer"),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        size: std::mem::size_of::<MaterialUniform>() as wgpu::BufferAddress,
        mapped_at_creation: false,
    });

    let material_bind_group = deps.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &deps.material_uniform_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                // TODO: imported texture
                resource: wgpu::BindingResource::TextureView(&deps.white_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                // TODO: imported sampler
                resource: wgpu::BindingResource::Sampler(&deps.white_texture.sampler),
            }
        ],
        label: Some("material_bind_group"),
    });

    deps.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[material_uniform]));

    model::Material {
        gltf_index: material.index().unwrap(),
        base_color_factor,
        emissive_factor,
        material_bind_group,
        uniform_buffer,
    }
}

fn import_mesh(mesh: gltf::Mesh, root: &GltfRoot, deps: &WgpuDeps) -> model::Mesh {
    model::Mesh {
        gltf_index: mesh.index(),
        primitives: mesh
            .primitives()
            .map(|p| import_primitive(p, root, deps))
            .collect(),
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    root: &GltfRoot,
    deps: &WgpuDeps,
) -> Option<model::MeshPrimitive> {
    use gltf::mesh::*;

    let index = primitive.index();

    let mode = primitive.mode();
    if mode != Mode::Triangles {
        eprintln!("Primitive {} is not of triangles mode. Skip", index);
        return None;
    }
    let index_acc = primitive.indices().expect("Failed to get index accessor");
    let (index_buffer, index_size) = import_buffer(
        &index_acc,
        root,
        deps,
        None,
        "Vertex Index",
        wgpu::BufferUsages::INDEX,
    ).expect("Failed to get index buffer");
    let index_format = match index_size {
        2 => wgpu::IndexFormat::Uint16,
        4 => wgpu::IndexFormat::Uint32,
        _ => panic!("Unsupported index format"),
    };

    let position_acc = primitive.get(&Semantic::Positions).expect("Failed to get position accessor");
    let normal_acc = primitive.get(&Semantic::Normals).expect("Failed to get normal accessor");
    let tex_coord_acc = primitive.get(&Semantic::TexCoords(0));

    let vertex_count = position_acc.count();

    let position_buffer = import_buffer(
        &position_acc,
        root,
        deps,
        Some(12),
        "Vertex Position",
        wgpu::BufferUsages::VERTEX,
    ).unwrap().0;

    let normal_buffer = import_buffer(
        &normal_acc,
        root,
        deps,
        Some(12),
        "Vertex Normal",
        wgpu::BufferUsages::VERTEX,
    ).unwrap().0;

    let tex_coord_buffer = tex_coord_acc.map(|acc| import_buffer(
        &acc,
        root,
        deps,
        Some(8),
        "Vertex Tex Coord",
        wgpu::BufferUsages::VERTEX,
    ).unwrap().0).unwrap_or_else(|| {
        log::warn!("Creating null texture coordiates buffer");
        create_null_texcoord_buffer(deps, vertex_count)
    });

    Some(model::MeshPrimitive {
        gltf_index: index,
        material_id: primitive.material().index(),
        position_buffer,
        normal_buffer,
        tex_coord_buffer,
        index_buffer,
        index_format,
        num_indices: index_acc.count(),
    })
}

fn import_buffer(
    acc: &gltf::Accessor,
    root: &GltfRoot,
    deps: &WgpuDeps,
    assert_stride: Option<usize>,
    label: &str,
    usage: wgpu::BufferUsages,
) -> Option<(wgpu::Buffer, usize)> {
    let view = acc.view().expect("Failed to load buffer view from accessor");

    let stride = view.stride().unwrap_or_else(|| acc.size());
    if let Some(assert_stride) = assert_stride {
        if stride != assert_stride {
            panic!("Buffer is not tightly-packed or has invalid type");
        }
    }

    let offset = view.offset() + acc.offset();
    let length = acc.size() * acc.count();
    let buffer = &root.buffers[view.buffer().index()].0;
    let slice = &buffer[offset..(offset + length)];
    let wgpu_buffer = deps.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: slice,
        usage,
    });
    Some((wgpu_buffer, stride))
}

// TODO: shader permutation or pipeline overridable constants
fn create_null_texcoord_buffer(
    deps: &WgpuDeps,
    count: usize,
) -> wgpu::Buffer {
    let mut data = Vec::new();
    data.resize(count * 2, 0.0f32);
    let raw_data = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
    };
    deps.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Null texture coordidates"),
        contents: raw_data,
        usage: wgpu::BufferUsages::VERTEX,
    })
}
