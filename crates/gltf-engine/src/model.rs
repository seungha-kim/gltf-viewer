use crate::mesh::Mesh;
use cgmath::*;
use std::collections::HashMap;
use uuid::Uuid;

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
    pub default_scene_id: Option<Uuid>,
    pub scenes: HashMap<Uuid, Scene>,
    pub nodes: HashMap<Uuid, Node>,
    pub meshes: HashMap<Uuid, Mesh>,
    pub materials: HashMap<Uuid, Material>,
}

impl ImportedGltf {
    pub fn default_scene(&self) -> &Scene {
        if let Some(default_scene_id) = self.default_scene_id {
            &self.scenes[&default_scene_id]
        } else {
            self.scenes.iter().nth(0).map(|(_id, s)| s).unwrap()
        }
    }
}

#[derive(Debug)]
pub enum SceneSourceInfo {
    Gltf { index: usize },
    SomethingElse,
}

#[derive(Debug)]
pub struct Scene {
    pub id: Uuid,
    pub nodes: Vec<Uuid>,
    pub source_info: SceneSourceInfo,
}

impl Scene {
    pub fn gltf_index(&self) -> usize {
        let SceneSourceInfo::Gltf { index } = self.source_info else {
            panic!("Source is not glTF");
        };
        index
    }
}

#[derive(Debug)]
pub enum NodeSourceInfo {
    Gltf { index: usize },
    SomethingElse,
}

#[derive(Debug)]
pub struct NodeTransform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl NodeTransform {
    pub fn matrix(&self) -> Matrix4<f32> {
        let translation_mat = Matrix4::from_translation(self.position.into());
        let rotation_mat: Matrix4<f32> = self.rotation.into();
        let scale_mat = Matrix4::from_nonuniform_scale(self.scale[0], self.scale[1], self.scale[2]);
        translation_mat * rotation_mat * scale_mat
    }
}

#[derive(Debug)]
pub struct Node {
    pub id: Uuid,
    pub transform: NodeTransform,
    pub children: Vec<Uuid>,
    pub mesh_id: Option<Uuid>,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub source_info: NodeSourceInfo,
}

impl Node {
    pub fn gltf_index(&self) -> usize {
        let NodeSourceInfo::Gltf { index } = self.source_info else {
            panic!("Source is not glTF");
        };
        index
    }
}

pub enum MaterialSourceInfo {
    Gltf { index: usize },
    SomethingElse,
}

pub struct Material {
    pub id: Uuid,
    pub base_color_factor: Vector4<f32>,
    pub emissive_factor: Vector3<f32>,

    pub uniform_buffer: wgpu::Buffer,
    pub material_bind_group: wgpu::BindGroup,

    pub source_info: MaterialSourceInfo,
    // TODO: enum
}

impl Material {
    pub fn gltf_index(&self) -> usize {
        let MaterialSourceInfo::Gltf { index } = self.source_info else {
            panic!("Source is not glTF");
        };
        index
    }
}
