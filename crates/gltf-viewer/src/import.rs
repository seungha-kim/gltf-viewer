use crate::model;
use wgpu::util::DeviceExt;

pub struct GltfRoot {
    pub document: gltf::Document,
    pub buffers: Vec<gltf::buffer::Data>,
    pub images: Vec<gltf::image::Data>,
}

pub fn import_gltf(root: &GltfRoot, device: &wgpu::Device) -> model::ModelRoot {
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
        .map(import_node)
        .collect();

    let meshes = document
        .meshes()
        .map(|mesh| import_mesh(mesh, root, device))
        .collect();

    model::ModelRoot {
        default_scene_id,
        scenes,
        nodes,
        meshes,
        materials: Vec::new(),
    }
}

fn import_scene(scene: gltf::Scene) -> model::Scene {
    let id = scene.index();
    let mut nodes = Vec::new();
    for root_node in scene.nodes() {
        nodes.push(root_node.index());
    }
    model::Scene { id, nodes }
}

fn import_node(node: gltf::Node) -> model::Node {
    model::Node {
        id: node.index(),
        transform: import_transform(node.transform()),
        children: node.children().map(|child| child.index()).collect(),
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

fn import_mesh(mesh: gltf::Mesh, root: &GltfRoot, device: &wgpu::Device) -> model::Mesh {
    model::Mesh {
        id: mesh.index(),
        primitives: mesh
            .primitives()
            .map(|p| import_primitive(p, root, device))
            .collect(),
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    root: &GltfRoot,
    device: &wgpu::Device,
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

    Some(model::MeshPrimitive {
        id: index,
        material_id: primitive.material().index(),
        position_buffer: import_buffer(position_view, root, device, "Vertex Position", wgpu::BufferUsages::VERTEX),
        index_buffer: import_buffer(index_view, root, device, "Vertex Index", wgpu::BufferUsages::INDEX),
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
