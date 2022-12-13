use std::{
    fs::File,
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::NonZeroU32,
    rc::Rc, hash::{Hash, Hasher, BuildHasher}, collections::{hash_map::{DefaultHasher, RandomState}, HashMap},
};

use anyhow::Result;
use cubescript2_macros::derive_desc;
use image::DynamicImage;
use memoffset::offset_of;
use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Projective3, Vector3};
use once_cell::sync::OnceCell;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::{utils::*, window::Input};

use super::*;

pub trait Vertex {}

type VertexAttributeLayoutOwnerId = u64;

#[derive(Debug)]
pub struct MeshManager {
    pub attr_lay_set: HashMap<VertexAttributeLayoutOwnerId,VertexAttributeLayoutOwner>,
    pub cube_mesh: Mesh<cube::CubeVertx>,
    pub cube_mesh_bind: MeshBind,
}

impl MeshManager {
    pub fn init(device: &Device) -> MeshManager {
        let mut attr_lay_set = HashMap::<_,_,RandomState>::default();
        let attr_lay = cube::CubeVertx::attr_desc();
        let h = {
            let mut hasher = attr_lay_set.hasher().build_hasher();
            attr_lay.hash(&mut hasher);
            hasher.finish()
        };
        attr_lay_set.insert(h,attr_lay);
        
        let cube_mesh = Mesh::<cube::CubeVertx> {
            vert: cube::TEST_VERTICES.into(),
            indices: cube::TEST_INDICES.into(),
            attr_lay_id: h,
        };
        let mut builder = MeshBindBuilder::<cube::CubeVertx>::default();
        builder.set_device(device).set_label("Cube").set_mesh(&cube_mesh);
        let cube_mesh_bind = builder.build();

        MeshManager {
            attr_lay_set,
            cube_mesh,
            cube_mesh_bind,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VertexAttributeLayoutOwner {
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Debug)]
pub struct Mesh<V> {
    pub vert: Vec<V>,
    pub indices: Vec<u16>,
    pub attr_lay_id: VertexAttributeLayoutOwnerId,
}

#[derive(Debug)]
pub struct MeshBind {
    pub vertex_bind: Buffer,
    pub index_bind: Buffer,
    pub index_format: IndexFormat,
}
impl MeshBind {}

pub const VERT_ATTR_LABEL: &'static str = " Vertex Attribute";
pub const INDEX_LABEL: &'static str = " Vertex Index";
#[derive(Default)]
pub struct MeshBindBuilder<'a,V> {
    device: Option<&'a Device>,
    label: Option<&'a str>,
    mesh: Option<&'a Mesh<V>>,
}

impl<'a,V> MeshBindBuilder<'a,V> 
where V:bytemuck::Pod
{
    builder_set_fn!(set_device,device,&'a Device);
    builder_set_fn!(set_label, label, &'a str);
    builder_set_fn!(set_mesh, mesh, &'a Mesh<V>);

    fn get_label_or_default(&self) -> Option<&'a str> {
        self.label.or_else(|| Some("Unnamed"))
    }

    pub fn build(mut self) -> MeshBind {
        let device = self.device.unwrap();
        let label = self.get_label_or_default();

        let vert_label = label.map(|s| {
            let mut s = s.to_string();
            s.push_str(VERT_ATTR_LABEL);
            s
        });
        let index_label = label.map(|s| {
            let mut s = s.to_string();
            s.push_str(INDEX_LABEL);
            s
        });
        let vertex_bind = device.create_buffer_init(&BufferInitDescriptor {
            label: vert_label.as_deref(),
            contents: bytemuck::cast_slice(&self.mesh.unwrap().vert[..]),
            usage: BufferUsages::VERTEX,
        });
        let index_bind = device.create_buffer_init(&BufferInitDescriptor{
            label:index_label.as_deref(),
            contents: bytemuck::cast_slice(&self.mesh.unwrap().indices[..]),
            usage: BufferUsages::INDEX,
        });
        MeshBind {
            vertex_bind: vertex_bind,
            index_bind: index_bind,
            index_format: IndexFormat::Uint16,
        }
    }
}
