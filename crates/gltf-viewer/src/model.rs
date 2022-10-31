use cgmath::prelude::*;
use cgmath::{Matrix4, Vector3};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Id(usize);

impl From<usize> for Id {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

struct ModelAsset {
    default_scene_id: Id,
    scenes: HashMap<Id, Node>,
    objects: HashMap<Id, Node>,
    meshes: HashMap<Id, Mesh>,
    prims: HashMap<Id, MeshPrim>,
    materials: HashMap<Id, Material>,
}

struct Scene {
    id: Id,
}

struct Node {
    id: Id,
    transform: Matrix4<f32>,
}

struct Mesh {
    id: Id,
    prims: Vec<Id>,
}

struct MeshPrim {
    id: Id,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

struct Material {
    id: Id,
}
