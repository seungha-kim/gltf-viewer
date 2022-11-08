use cgmath::*;

/*
런타임에 바뀔 수 있는 것과 바뀔 수 없는 것 구분
바뀔 수 있는 것
- node transform
- node hierarchy
- current scene

바뀔 수 없는 것
- material properties (추후 변경 가능하게, 지금은 불변으로)
- texture
- mesh

texture bind group 은 material 에 있는게 맞...나?
*/

// 여러가지 use case 들이 생각나서 설계를 할 때 머리가 복잡해지네
// 정확한 use case 를 정하자. 범용 어쩌고는 만들 생각 하지마!
// 일단 MVP 정도만 생각하자.
// "glTF 포맷 파일 하나를" "아무런 편집 기능 없이" "애니메이션을 제외하고" "잘 보여주는" 앱
// 씬 전환 가능
// hierarchy 열람 가능
// node 선택 + highlight 가능

pub struct ImportedGltf {
    pub default_scene_id: usize,
    pub scenes: Vec<Scene>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Debug)]
pub struct Scene {
    pub gltf_index: usize,
    pub nodes: Vec<usize>,
}

#[derive(Debug)]
pub struct Node {
    pub gltf_index: usize,
    pub transform: Matrix4<f32>,
    pub children: Vec<usize>,
    pub mesh_index: Option<usize>,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

pub struct Material {
    pub gltf_index: usize,
    pub base_color_factor: Vector4<f32>,
    pub emissive_factor: Vector3<f32>,

    pub uniform_buffer: wgpu::Buffer,
    pub material_bind_group: wgpu::BindGroup,
    // TODO: enum
}

pub struct Mesh {
    pub gltf_index: usize,
    pub primitives: Vec<Option<MeshPrimitive>>,
}

pub enum MeshPrimitiveVertexBuffer {
    SeparatedIndexed {
        position: wgpu::Buffer,
        normal: wgpu::Buffer,
        tex_coord_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        num_indices: usize,
    }
}

pub struct MeshPrimitive {
    pub gltf_index: usize,
    pub vertex_buffer: MeshPrimitiveVertexBuffer,
    pub material_id: Option<usize>,
}
