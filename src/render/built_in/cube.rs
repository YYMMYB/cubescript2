use std::{
    fs::File,
    hash::Hash,
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::{NonZeroU32, NonZeroU64},
    rc::Rc,
};

use anyhow::Result;
use bytemuck::cast_slice;
use cubescript2_macros::derive_desc;
use image::DynamicImage;
use memoffset::offset_of;
use nalgebra::{Affine3, Isometry3, Matrix4, Perspective3, Point3, Projective3, Vector3};
use once_cell::sync::OnceCell;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::{
    logic::{
        orient::{CompressedData, Orient},
        *,
    },
    utils::*,
};

use super::super::*;

pub const TEST_VERTICES: &[CubeVertx] = &[
    CubeVertx {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    CubeVertx {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    CubeVertx {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    CubeVertx {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
];

pub const TEST_INDICES: &[u16] = &[2, 3, 0, 0, 1, 2];

pub const TEST_INSTANCES: &[CubeInstance] = &[
    CubeInstance {
        info: [2, 0, 0, 0],
        position: [0.0, 1.0, -3.0],
        color: [1.0, 0.1, 0.1],
    },
    CubeInstance {
        info: [1, 0, 16, 0],
        position: [0.0, 0.0, -2.0],
        color: [0.1, 1.0, 0.1],
    },
    CubeInstance {
        info: [0, 0, 32, 0],
        position: [0.0, -1.0, -1.0],
        color: [0.1, 0.1, 1.0],
    },
];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CubeVertx {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl CubeVertx {
    pub fn attr_desc() -> VertexAttributeLayoutOwner {
        let attributes = vertex_attribute_layout!(
            Self, struct, {
                0;position ; Float32x3,
                1;tex_coords ; Float32x2,
            }
        );
        VertexAttributeLayoutOwner {
            attributes: attributes.into(),
        }
    }
    pub fn desc(attr_lay: &VertexAttributeLayoutOwner) -> VertexBufferLayout<'_> {
        vertex_buffer_layout!(CubeVertx, Vertex, &attr_lay.attributes[..])
    }
}
impl VSVertex for CubeVertx {}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CubeInstance {
    info: [u8; 4], // [exp, texure_index, rotation_id(0..24), 0]
    position: [f32; 3],
    color: [f32; 3],
}

impl CubeInstance {
    pub fn attr_desc() -> VertexAttributeLayoutOwner {
        let attributes = vertex_attribute_layout!(Self, struct, {
            2;info ; Uint8x4,
            3;position ; Float32x3,
            4;color ; Float32x3,
        });
        VertexAttributeLayoutOwner {
            attributes: attributes.into(),
        }
    }
    pub fn desc(attr_lay: &VertexAttributeLayoutOwner) -> VertexBufferLayout<'_> {
        vertex_buffer_layout!(CubeInstance, Instance, &attr_lay.attributes[..])
    }
}
impl VSInstance for CubeInstance {}

const ORIENT_COUNT: usize = 48;
#[derive(Debug)]
pub struct ConstResource {
    pub rot_mat: [[[f32; 4]; 4]; 48],
}

impl ConstResource {
    pub fn init() -> Self {
        let rot: [[f32; 4]; 4] = Default::default();
        let mut rot_mat: [[[f32; 4]; 4]; ORIENT_COUNT] = [rot; ORIENT_COUNT];
        for code in 0..ORIENT_COUNT as u8 {
            let orint = Orient::<CompressedData>::decode(code).uncompress();
            let mat = orint.to_matrix();
            let mat = mat.to_homogeneous();
            rot_mat[code as usize] = mat.into();
        }
        Self { rot_mat }
    }
    pub fn create_bind(&self, device: &Device) -> ConstResourceBind {
        let rot_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Cube Resource Rot Matrix"),
            contents: cast_slice(&self.rot_mat),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        ConstResourceBind {
            rot_mat: rot_buffer,
        }
    }
}

const SPLIT_COUNT: usize = 4;
#[derive(Debug)]
pub struct ConstResourceBind {
    pub rot_mat: Buffer,
}

impl ConstResourceBind {
    pub fn get_entries_desc<'a>(
        &'a self,
    ) -> [BindGroupBuilderEntryDesc<'a>; 1] {
        let rot_desc = BindGroupBuilderEntryDesc {
            resource: self.rot_mat.as_entire_binding(),
            visibility: ShaderStages::VERTEX,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        [rot_desc]
    }

    pub fn write(&self, queue: &Queue, data: &ConstResource) {
        queue.write_buffer(&self.rot_mat, 0, cast_slice(&data.rot_mat));
    }
}

pub fn build_bind_group(
    device: &Device,
    const_resource_bind: &ConstResourceBind,
) -> Result<(BindGroupLayout, BindGroup)> {
    let mut builder = BindGroupBuider::default();
    let descs = const_resource_bind.get_entries_desc();
    builder
        .set_device(device)
        .set_label("Cube")
        .push_entries(descs);
    let (layout, group) = builder.build()?;
    Ok((layout, group))
}
