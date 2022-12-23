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

use crate::utils::*;

use super::*;


pub struct BindGroupLayoutEntryArgs {
    pub count: Option<NonZeroU32>,
    pub visibility: ShaderStages,
    pub ty: BindingType,
}
pub fn create_bind_group_layout<'a, A>(
    device: &'a Device,
    label: Option<&'a str>,
    args: A,
) -> Result<BindGroupLayout>
where
    A: IntoIterator<Item = &'a BindGroupLayoutEntryArgs>,
{
    let mut entries = Vec::new();
    let mut i = 0;
    for arg in args {
        entries.push(BindGroupLayoutEntry {
            binding: i,
            visibility: arg.visibility,
            ty: arg.ty,
            count: arg.count,
        });
        i += 1;
    }
    let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: label,
        entries: &entries[..],
    });
    Ok(layout)
}

pub fn create_bind_group<'a, A>(
    device: &'a Device,
    label: Option<&'a str>,
    layout: &'a BindGroupLayout,
    args: A,
) -> Result<BindGroup>
where
    A: IntoIterator<Item = &'a BindingResource<'a>>,
{
    let mut entries = Vec::new();
    let mut i = 0;
    for arg in args {
        entries.push(BindGroupEntry {
            binding: i,
            resource: arg.clone(),
        });
        i += 1;
    }
    let group = device.create_bind_group(&BindGroupDescriptor {
        label,
        layout,
        entries: &entries[..],
    });
    Ok(group)
}

pub fn create_shader_module(
    device: &Device,
    label: Option<&str>,
    source: ShaderSource,
) -> Result<ShaderModule> {
    Ok(device.create_shader_module(ShaderModuleDescriptor { label, source }))
}

pub fn create_pipeline_layout<'a>(
    device: &'a Device,
    label: Option<&'a str>,
    group_layouts: impl IntoIterator<Item = &'a BindGroupLayout>,
) -> Result<PipelineLayout> {
    let group_layouts = Vec::from_iter(group_layouts);
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label,
        bind_group_layouts: &group_layouts[..],
        push_constant_ranges: &[],
    });
    Ok(layout)
}

pub fn create_buffer<T>(device: &Device, label: Option<&str>, usage:BufferUsages,contents:&[T]) -> Buffer
where T:bytemuck::Pod
{
    device.create_buffer_init(&BufferInitDescriptor{
        label,
        contents: bytemuck::cast_slice(contents),
        usage,
    })
}

const BUILDER_FIELD_UNSET: &'static str = "builder 必须字段未被设置";
