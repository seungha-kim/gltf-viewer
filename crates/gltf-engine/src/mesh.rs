use uuid::Uuid;

pub enum MeshSourceInfo {
    Gltf { index: usize },
    SomethingElse,
}

pub struct Mesh {
    pub id: Uuid,
    pub primitives: Vec<Option<MeshPrimitive>>,
    pub source_info: MeshSourceInfo,
}

impl Mesh {
    pub fn gltf_index(&self) -> usize {
        let MeshSourceInfo::Gltf { index } = self.source_info else {
            panic!("Source is not glTF");
        };
        index
    }
}

pub enum PrimitiveSourceInfo {
    Gltf { index: usize },
}

pub struct MeshPrimitive {
    pub id: Uuid,
    pub position_buffer: wgpu::Buffer,
    pub normal_buffer: wgpu::Buffer,
    pub tex_coord_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: usize,
    pub index_format: wgpu::IndexFormat,
    pub material_id: Option<Uuid>,
    pub source_info: PrimitiveSourceInfo,
}
