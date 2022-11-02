use cgmath::*;
use crate::texture::Texture;

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

pub struct Mesh {
    pub id: usize,
    pub primitives: Vec<Option<MeshPrimitive>>,
}

pub enum MeshPrimitiveBuffer {

}

pub struct MeshPrimitive {
    pub id: usize,
    pub position_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub material_id: Option<usize>,
    // vertex_position_buffer: wgpu::Buffer,
    // vertex_tex_coord_buffer: wgpu::Buffer,
    // index_buffer: wgpu::Buffer,
    // num_indices: u32,
}

pub struct Material {
    pub id: usize,
    pub texture: Option<Texture>
}
