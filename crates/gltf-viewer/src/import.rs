use cgmath::{Matrix, SquareMatrix};
use crate::{model, NodeUniform};
use wgpu::util::DeviceExt;
use crate::model::MeshPrimitiveVertexBuffer;

pub struct GltfRoot {
    pub document: gltf::Document,
    pub buffers: Vec<gltf::buffer::Data>,
    pub images: Vec<gltf::image::Data>,
}

pub struct WgpuDeps<'a> {
    pub device: &'a wgpu::Device,
    pub node_uniform_layout: &'a wgpu::BindGroupLayout,
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

    model::ImportedGltf {
        default_scene_id,
        scenes,
        nodes,
        meshes,
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

    let position_acc = if let Some(position_acc) = primitive.get(&Semantic::Positions) {
        position_acc
    } else {
        eprintln!("Primitive {} has no positions. Skip", index);
        return None;
    };

    let position_view = if let Some(position_view) = position_acc.view() {
        position_view
    } else {
        eprintln!("Primitive {} has sparse position view. Skip", index);
        return None;
    };

    if position_view.stride().is_some() {
        eprintln!(
            "Primitive {} has position buffer not tightly-packed. Skip",
            index
        );
        return None;
    }

    let normal_acc = if let Some(normal_acc) = primitive.get(&Semantic::Normals) {
        normal_acc
    } else {
        eprintln!("Primitive {} has no normal buffer. Skip", index);
        return None;
    };

    let normal_view = if let Some(normal_view) = normal_acc.view() {
        normal_view
    } else {
        eprintln!("Primitive {} has sparse normal view. Skip", index);
        return None;
    };

    if normal_view.stride().is_some() {
        eprintln!(
            "Primitive {} has normal buffer not tightly-packed. Skip",
            index
        );
        return None;
    }

    let tex_coord_acc = if let Some(tex_coord_acc) = primitive.get(&Semantic::TexCoords(0)) {
        tex_coord_acc
    } else {
        eprintln!("Primitive {} has no 0th texture coordinate. Skip", index);
        return None;
    };

    let tex_coord_view = if let Some(tex_coord_view) = tex_coord_acc.view() {
        tex_coord_view
    } else {
        eprintln!("Primitive {} has sparse texture coordinate view. Skip", index);
        return None;
    };

    if tex_coord_view.stride().is_some() {
        eprintln!(
            "Primitive {} has texture coordinate buffer not tightly-packed. Skip",
            index
        );
        return None;
    }

    let index_acc = if let Some(index_acc) = primitive.indices() {
        index_acc
    } else {
        eprintln!("Primitive {} has no indices. Skip", index);
        return None;
    };

    let index_view = if let Some(index_view) = index_acc.view() {
        index_view
    } else {
        eprintln!("Primitive {} has sparse index view. Skip", index);
        return None;
    };

    if index_view.stride().is_some() {
        eprintln!(
            "Primitive {} has index buffer not tightly-packed. Skip",
            index
        );
        return None;
    }

    let device = deps.device;

    Some(model::MeshPrimitive {
        gltf_index: index,
        material_id: primitive.material().index(),
        vertex_buffer: MeshPrimitiveVertexBuffer::SeparatedIndexed {
            position: import_buffer(position_view, root, device, "Vertex Position", wgpu::BufferUsages::VERTEX),
            normal: import_buffer(normal_view, root, device, "Vertex Normal", wgpu::BufferUsages::VERTEX),
            tex_coord_buffer: import_buffer(tex_coord_view, root, device, "Vertex Tex Coord", wgpu::BufferUsages::VERTEX),
            index_buffer: import_buffer(index_view, root, device, "Vertex Index", wgpu::BufferUsages::INDEX),
            num_indices: index_acc.count(),
        },
    })
}

fn import_buffer(
    view: gltf::buffer::View,
    root: &GltfRoot,
    device: &wgpu::Device,
    label: &str,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    // TODO: non tightly-packed buffers
    assert!(view.stride().is_none());
    let offset = view.offset();
    let length = view.length();
    let buffer = &root.buffers[view.buffer().index()].0;
    let slice = &buffer[offset..(offset + length)];
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: slice,
        usage,
    })
}
