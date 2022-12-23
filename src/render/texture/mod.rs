use std::{
    fs::File,
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

use crate::utils::builder_set_fn;
use resource::*;

use self::format_info::TextureFormatPixelInfo;

use super::*;

pub mod format_info;

#[derive(Debug)]
pub struct TextureBind {
    pub texture: wgpu::Texture,
    pub view: TextureView,
}

impl TextureBind {}

pub struct TextureArgs {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsages,
}
impl TextureArgs {
    pub fn depth_texture() -> TextureArgs {
        TextureArgs {
            width: 0,
            height: 0,
            depth: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT,
        }
    }

    pub fn texture_array() -> TextureArgs {
        TextureArgs {
            width: 0,
            height: 0,
            depth: 0,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
        }
    }

    pub fn into_desc(self, label: Option<&str>) -> TextureDescriptor {
        TextureDescriptor {
            label,
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: self.depth,
            },
            mip_level_count: self.mip_level_count,
            sample_count: self.sample_count,
            dimension: self.dimension,
            format: self.format,
            usage: self.usage,
        }
    }
}
