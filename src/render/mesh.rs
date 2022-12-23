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
use image::DynamicImage;
use memoffset::offset_of;
use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Projective3, Vector3};
use once_cell::sync::OnceCell;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::utils::*;

use super::*;

pub trait VSVertex {}
pub trait VSInstance {}

type VertexAttributeLayoutOwnerId = u64;

#[derive(Debug)]
pub struct MeshManager {
    pub attr_lay_set: HashMap<VertexAttributeLayoutOwnerId, VertexAttributeLayoutOwner>,
    pub cube_meshs: Vec<Mesh<cube::CubeVertx, cube::CubeInstance>>,
    pub cube_mesh_binds: Vec<MeshBind>,
}

impl MeshManager {
    pub fn init(device: &Device) -> Result<MeshManager> {
        let mut layout_set = HashMap::<_, _, RandomState>::default();
        let v_lay = cube::CubeVertx::attr_desc();
        let hv = {
            let mut hasher = layout_set.hasher().build_hasher();
            v_lay.hash(&mut hasher);
            hasher.finish()
        };
        layout_set.insert(hv, v_lay);
        let i_lay = cube::CubeInstance::attr_desc();
        let hi = {
            let mut hasher = layout_set.hasher().build_hasher();
            i_lay.hash(&mut hasher);
            hasher.finish()
        };
        layout_set.insert(hi, i_lay);

        let mut cube_meshs = Vec::new();
        let mut cube_mesh_binds = Vec::new();
        const N: i32 = 5;
        for x in -N..N {
            for y in -N..N {
                for z in -N..N {
                    let inss : Vec<_> = cube::TEST_INSTANCES.iter().map(|ins|{
                        let mut ins = ins.clone();
                        let p = ins.position;
                        ins.position = [
                            p[0] + x as f32 * 3f32,
                            p[1] + y as f32 * 3f32,
                            p[2] + z as f32 * 3f32,
                        ];
                        ins
                    }).collect();
                    let cube_mesh = Mesh::<cube::CubeVertx, cube::CubeInstance> {
                        vert: cube::TEST_VERTICES.into(),
                        indices: cube::TEST_INDICES.into(),
                        instance: inss.into(),
                        vertex_layout_id: hv,
                        instance_layout_id: hi,
                        index_format: IndexFormat::Uint16,
                    };
                    let mut builder =
                        MeshBindBuilder::<cube::CubeVertx, cube::CubeInstance>::default();
                    builder
                        .set_device(device)
                        .set_label("Cube")
                        .set_mesh(&cube_mesh);
                    let cube_mesh_bind = builder.build()?;
                    cube_meshs.push(cube_mesh);
                    cube_mesh_binds.push(cube_mesh_bind);
                }
            }
        }

        Ok(MeshManager {
            attr_lay_set: layout_set,
            cube_meshs,
            cube_mesh_binds,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VertexAttributeLayoutOwner {
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Debug)]
pub struct Mesh<V, I> {
    pub vert: Vec<V>,
    pub indices: Vec<u16>,
    pub instance: Vec<I>,
    pub vertex_layout_id: VertexAttributeLayoutOwnerId,
    pub instance_layout_id: VertexAttributeLayoutOwnerId,
    pub index_format: IndexFormat,
}

#[derive(Debug)]
pub struct MeshBind {
    pub vertex_bind: Buffer,
    pub index_bind: Buffer,
    pub instance_bind: Buffer,
}
impl MeshBind {}

#[derive(Default)]
pub struct MeshBindBuilder<'a, V, I> {
    device: Option<&'a Device>,
    label: Option<&'a str>,
    mesh: Option<&'a Mesh<V, I>>,
}

impl<'a, V, I> MeshBindBuilder<'a, V, I>
where
    V: bytemuck::Pod,
    I: bytemuck::Pod,
{
    builder_set_fn!(set_device, device, &'a Device);
    builder_set_fn!(set_label, label, &'a str);
    builder_set_fn!(set_mesh, mesh, &'a Mesh<V, I>);

    pub fn build(mut self) -> Result<MeshBind> {
        let device = self.device.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let mesh = self.mesh.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let label = &self.label;

        let vert_label = get_default_label(label, [VERT_ATTR_LABEL]);
        let index_label = get_default_label(label, [INDEX_LABEL]);
        let instance_label = get_default_label(label, [INSTANCE_LABEL]);
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
        let instance_bind = device.create_buffer_init(&BufferInitDescriptor {
            label: instance_label.as_deref(),
            contents: bytemuck::cast_slice(&mesh.instance[..]),
            usage: BufferUsages::VERTEX,
        });
        Ok(MeshBind {
            vertex_bind: vertex_bind,
            index_bind: index_bind,
            instance_bind: instance_bind,
        })
    }
}

const BUILDER_FIELD_UNSET: &'static str = "builder 必须字段未被设置";
