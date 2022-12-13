use std::{
    collections::{
        hash_map::{DefaultHasher, RandomState},
        HashMap,
    },
    fs::File,
    hash::{BuildHasher, Hash, Hasher},
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::NonZeroU32,
    rc::Rc,
};

use anyhow::*;
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
    pub attr_lay_set: HashMap<VertexAttributeLayoutOwnerId, VertexAttributeLayoutOwner>,
    pub cube_mesh: Mesh<cube::CubeVertx>,
    pub cube_mesh_bind: MeshBind,
}

impl MeshManager {
    pub fn init(device: &Device) -> Result<MeshManager> {
        let mut attr_lay_set = HashMap::<_, _, RandomState>::default();
        let attr_lay = cube::CubeVertx::attr_desc();
        let h = {
            let mut hasher = attr_lay_set.hasher().build_hasher();
            attr_lay.hash(&mut hasher);
            hasher.finish()
        };
        attr_lay_set.insert(h, attr_lay);

        let cube_mesh = Mesh::<cube::CubeVertx> {
            vert: cube::TEST_VERTICES.into(),
            indices: cube::TEST_INDICES.into(),
            attr_lay_id: h,
            index_format: IndexFormat::Uint16,
        };
        let mut builder = MeshBindBuilder::<cube::CubeVertx>::default();
        builder
            .set_device(device)
            .set_label("Cube")
            .set_mesh(&cube_mesh);
        let cube_mesh_bind = builder.build()?;

        Ok(MeshManager {
            attr_lay_set,
            cube_mesh,
            cube_mesh_bind,
        })
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
    pub index_format: IndexFormat,
}

#[derive(Debug)]
pub struct MeshBind {
    pub vertex_bind: Buffer,
    pub index_bind: Buffer,
}
impl MeshBind {}

#[derive(Default)]
pub struct MeshBindBuilder<'a, V> {
    device: Option<&'a Device>,
    label: Option<&'a str>,
    mesh: Option<&'a Mesh<V>>,
}

impl<'a, V> MeshBindBuilder<'a, V>
where
    V: bytemuck::Pod,
{
    builder_set_fn!(set_device, device, &'a Device);
    builder_set_fn!(set_label, label, &'a str);
    builder_set_fn!(set_mesh, mesh, &'a Mesh<V>);

    pub fn build(mut self) -> Result<MeshBind> {
        let device = self.device.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let mesh = self.mesh.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let label = &self.label;

        let vert_label = get_default_label(label, [VERT_ATTR_LABEL]);
        let index_label = get_default_label(label, [INDEX_LABEL]);
        let vertex_bind = device.create_buffer_init(&BufferInitDescriptor {
            label: vert_label.as_deref(),
            contents: bytemuck::cast_slice(&mesh.vert[..]),
            usage: BufferUsages::VERTEX,
        });
        let index_bind = device.create_buffer_init(&BufferInitDescriptor {
            label: index_label.as_deref(),
            contents: bytemuck::cast_slice(&mesh.indices[..]),
            usage: BufferUsages::INDEX,
        });
        Ok(MeshBind {
            vertex_bind: vertex_bind,
            index_bind: index_bind,
        })
    }
}

const BUILDER_FIELD_UNSET: &'static str = "builder 必须字段未被设置";
