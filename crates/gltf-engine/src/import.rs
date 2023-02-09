use crate::mesh::*;
use crate::model::*;
use crate::texture;
use crate::*;
use crate::{MaterialUniform, NodeUniform};
use std::collections::HashMap;
use uuid::Uuid;
use wgpu::util::DeviceExt;

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

pub fn import_gltf(root: &GltfRoot, deps: &WgpuDeps) -> ImportedGltf {
    let document = &root.document;

    let materials: HashMap<Uuid, Material> = document
        .materials()
        .map(|m| import_material(m, deps))
        .map(|m| (m.id, m))
        .collect();

    let material_ids: HashMap<usize, Uuid> =
        materials.values().map(|m| (m.gltf_index(), m.id)).collect();

    let meshes: HashMap<Uuid, Mesh> = document
        .meshes()
        .map(|mesh| import_mesh(mesh, root, deps, &material_ids))
        .map(|mesh| (mesh.id, mesh))
        .collect();

    let mesh_ids: HashMap<usize, Uuid> = meshes.values().map(|m| (m.gltf_index(), m.id)).collect();

    let node_ids: HashMap<usize, Uuid> = root
        .document
        .nodes()
        .map(|n| (n.index(), Uuid::new_v4()))
        .collect();

    let nodes: HashMap<Uuid, Node> = document
        .nodes()
        .map(|n| import_node(n, deps, &mesh_ids, &node_ids))
        .map(|n| (n.id, n))
        .collect();

    let scenes: HashMap<Uuid, Scene> = document
        .scenes()
        .map(|scene| import_scene(scene, &node_ids))
        .map(|s| (s.id, s))
        .collect();

    let scene_ids: HashMap<usize, Uuid> = scenes.values().map(|s| (s.gltf_index(), s.id)).collect();

    let default_scene_id = document
        .default_scene()
        .map(|scene| scene_ids[&scene.index()]);

    // TODO: texture, sampler

    ImportedGltf {
        default_scene_id,
        scenes,
        nodes,
        meshes,
        materials,
    }
}

fn import_scene(scene: gltf::Scene, node_ids: &HashMap<usize, Uuid>) -> Scene {
    let mut nodes = Vec::new();
    for root_node in scene.nodes() {
        nodes.push(node_ids[&root_node.index()]);
    }
    Scene {
        id: Uuid::new_v4(),
        nodes,
        source_info: SceneSourceInfo::Gltf {
            index: scene.index(),
        },
    }
}

fn import_node(
    node: gltf::Node,
    deps: &WgpuDeps,
    mesh_ids: &HashMap<usize, Uuid>,
    node_ids: &HashMap<usize, Uuid>,
) -> Node {
    let transform = import_transform(node.transform());

    let uniform_buffer = deps.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Uniform Buffer"),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        size: std::mem::size_of::<NodeUniform>() as wgpu::BufferAddress,
        mapped_at_creation: false,
    });

    let uniform_bind_group = deps.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &deps.node_uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("primitive_transform_bind_group"),
    });

    Node {
        id: node_ids[&node.index()],
        transform,
        children: node
            .children()
            .map(|child| node_ids[&child.index()])
            .collect(),
        mesh_id: node.mesh().map(|m| mesh_ids[&m.index()]),
        uniform_buffer,
        uniform_bind_group,
        source_info: NodeSourceInfo::Gltf {
            index: node.index(),
        },
    }
}

fn import_transform(transform: gltf::scene::Transform) -> NodeTransform {
    use gltf::scene::Transform as G;
    match transform {
        G::Matrix { matrix } => {
            let mat4 = cgmath::Matrix4::from(matrix);
            let position_arr: [f32; 3] = [mat4.w.x, mat4.w.y, mat4.w.z];
            let position = Vector3::from(position_arr);

            let mut mat3 =
                Matrix3::from_cols(mat4.x.truncate(), mat4.y.truncate(), mat4.z.truncate());
            let mut scale =
                Vector3::new(mat3.x.magnitude(), mat3.y.magnitude(), mat3.z.magnitude());

            mat3.x /= scale.x;
            mat3.y /= scale.y;
            mat3.z /= scale.z;

            if mat3.determinant() < 0.0 {
                mat3 = -mat3;
                scale = -scale;
            }

            let rotation = cgmath::Quaternion::from(mat3);

            NodeTransform {
                position,
                rotation,
                scale,
            }
        }
        G::Decomposed {
            translation,
            rotation,
            scale,
        } => NodeTransform {
            position: translation.into(),
            rotation: rotation.into(),
            scale: scale.into(),
        },
    }
}

fn import_material(material: gltf::Material, deps: &WgpuDeps) -> Material {
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
            },
        ],
        label: Some("material_bind_group"),
    });

    deps.queue.write_buffer(
        &uniform_buffer,
        0,
        bytemuck::cast_slice(&[material_uniform]),
    );

    let source_info = MaterialSourceInfo::Gltf {
        index: material.index().unwrap(),
    };

    Material {
        id: Uuid::new_v4(),
        base_color_factor,
        emissive_factor,
        material_bind_group,
        uniform_buffer,
        source_info,
    }
}

fn import_mesh(
    mesh: gltf::Mesh,
    root: &GltfRoot,
    deps: &WgpuDeps,
    material_ids: &HashMap<usize, Uuid>,
) -> Mesh {
    Mesh {
        id: Uuid::new_v4(),
        primitives: mesh
            .primitives()
            .map(|p| import_primitive(p, root, deps, material_ids))
            .collect(),
        source_info: MeshSourceInfo::Gltf {
            index: mesh.index(),
        },
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    root: &GltfRoot,
    deps: &WgpuDeps,
    material_ids: &HashMap<usize, Uuid>,
) -> Option<MeshPrimitive> {
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
    )
    .expect("Failed to get index buffer");
    let index_format = match index_size {
        2 => wgpu::IndexFormat::Uint16,
        4 => wgpu::IndexFormat::Uint32,
        _ => panic!("Unsupported index format"),
    };

    let position_acc = primitive
        .get(&Semantic::Positions)
        .expect("Failed to get position accessor");
    let normal_acc = primitive
        .get(&Semantic::Normals)
        .expect("Failed to get normal accessor");
    let tex_coord_acc = primitive.get(&Semantic::TexCoords(0));

    let vertex_count = position_acc.count();

    let position_buffer = import_buffer(
        &position_acc,
        root,
        deps,
        Some(12),
        "Vertex Position",
        wgpu::BufferUsages::VERTEX,
    )
    .unwrap()
    .0;

    let normal_buffer = import_buffer(
        &normal_acc,
        root,
        deps,
        Some(12),
        "Vertex Normal",
        wgpu::BufferUsages::VERTEX,
    )
    .unwrap()
    .0;

    let tex_coord_buffer = tex_coord_acc
        .map(|acc| {
            import_buffer(
                &acc,
                root,
                deps,
                Some(8),
                "Vertex Tex Coord",
                wgpu::BufferUsages::VERTEX,
            )
            .unwrap()
            .0
        })
        .unwrap_or_else(|| {
            log::warn!("Creating null texture coordiates buffer");
            create_null_texcoord_buffer(deps, vertex_count)
        });

    Some(MeshPrimitive {
        id: Uuid::new_v4(),
        material_id: primitive.material().index().map(|i| material_ids[&i]),
        position_buffer,
        normal_buffer,
        tex_coord_buffer,
        index_buffer,
        index_format,
        num_indices: index_acc.count(),
        source_info: PrimitiveSourceInfo::Gltf { index: index },
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
    let view = acc
        .view()
        .expect("Failed to load buffer view from accessor");

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
    let wgpu_buffer = deps
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: slice,
            usage,
        });
    Some((wgpu_buffer, stride))
}

// TODO: shader permutation or pipeline overridable constants
fn create_null_texcoord_buffer(deps: &WgpuDeps, count: usize) -> wgpu::Buffer {
    let mut data = Vec::new();
    data.resize(count * 2, 0.0f32);
    let raw_data =
        unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };
    deps.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Null texture coordidates"),
            contents: raw_data,
            usage: wgpu::BufferUsages::VERTEX,
        })
}
