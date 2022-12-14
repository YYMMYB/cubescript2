use std::{
    fs::File,
    hash::Hash,
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::NonZeroU32,
    rc::Rc,
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

use crate::{utils::*};

use super::super::*;

pub const TEST_VERTICES: &[CubeVertx] = &[
    CubeVertx {
        position: [0.0, 0.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    CubeVertx {
        position: [1.0, 0.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    CubeVertx {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    CubeVertx {
        position: [0.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
];

pub const TEST_INDICES: &[u16] = &[2, 3, 0, 0, 1, 2];

pub const TEST_INSTANCES: &[CubeInstance] = &[
    CubeInstance {
        info: [2,0,0,0],
        position: [0.0,1.0,-3.0],
        color: [1.0,0.1,0.1],
    },
    CubeInstance {
        info: [1,0,0,0],
        position: [0.0,0.0,-2.0],
        color: [0.1,1.0,0.1],
    },
    CubeInstance {
        info: [0,0,0,0],
        position: [0.0,-1.0,-1.0],
        color: [0.1,0.1,1.0],
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
        let a: VertexFormat = VertexFormat::Uint8x4;
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
