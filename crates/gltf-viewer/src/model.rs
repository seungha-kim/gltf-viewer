use cgmath::*;

pub struct ModelRoot {
    pub default_scene_id: usize,
    pub scenes: Vec<Scene>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Debug)]
pub struct Scene {
    pub id: usize,
    pub nodes: Vec<usize>,
}

#[derive(Debug)]
pub struct Node {
    pub id: usize,
    pub transform: Matrix4<f32>,
    pub children: Vec<usize>,
}

#[derive(Debug)]
pub struct Mesh {
    pub id: usize,
    pub primitives: Vec<Option<MeshPrimitive>>,
}

#[derive(Debug)]
pub struct MeshPrimitive {
    pub id: usize,
    pub position_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub material_id: Option<usize>,
}

#[derive(Debug)]
pub struct Material {
    pub id: usize,
}
