
use std::{
    fs::File,
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


#[derive(Clone, Debug, Default)]
pub struct PipelineBuilder<'d> {
    pub device: Option<&'d Device>,
    pub layout: Option<&'d PipelineLayout>,
    pub vs_path: Option<&'d str>,
    pub fs_path: Option<&'d str>,
    pub label: Option<&'d str>,
    pub vertex_buffer: Option<&'d [VertexBufferLayout<'d>]>,
    pub target_format: Option<TextureFormat>,
    pub target_blend: Option<BlendState>,
    pub depth_format: Option<TextureFormat>,
    pub depth_write: Option<bool>,
}

impl<'d> PipelineBuilder<'d> {
    pub const VS_FUNC_NAME: &str = "vertex_main";
    pub const FS_FUNC_NAME: &str = "fragment_main";
    pub fn new() -> Self {
        let ret = Self::default();
        ret
    }

    builder_set_fn!(set_device, device, &'d Device);
    builder_set_fn!(set_layout, layout, &'d PipelineLayout);
    builder_set_fn!(set_vs_path, vs_path, &'d str);
    builder_set_fn!(set_fs_path, fs_path, &'d str);
    builder_set_fn!(set_label, label, &'d str);
    builder_set_fn!(
        set_vertex_buffer,
        vertex_buffer,
        &'d [VertexBufferLayout<'d>]
    );
    builder_set_fn!(set_target_format, target_format, TextureFormat);
    builder_set_fn!(set_target_blend, target_blend, BlendState);
    builder_set_fn!(set_depth_format, depth_format, TextureFormat);
    builder_set_fn!(set_depth_write, depth_write, bool);

    fn add_default_values(&mut self) {
        if self.target_blend.is_none() {
            self.set_target_blend(BlendState::REPLACE);
        };
        if self.depth_format.is_none() {
            self.set_depth_format(TextureFormat::Depth32Float);
        }
        if self.depth_write.is_none() {
            self.set_depth_write(true);
        }
    }

    pub fn build(mut self) -> Result<RenderPipeline> {
        self.add_default_values();

        let device = self.device.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let layout = self.layout.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let vs_path = self.vs_path.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let fs_path = self.fs_path.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let label = get_default_label(&self.label, [PIPELINE_LABEL]);
        let vertex_buffer = self.vertex_buffer.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let depth_format = self.depth_format.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let depth_write = self.depth_write.take().ok_or(anyhow!(BUILDER_FIELD_UNSET))?;

        let vs = {
            let mut f = File::open(vs_path)?;
            let mut s = String::new();
            f.read_to_string(&mut s);
            let desc = ShaderModuleDescriptor {
                label: Some(vs_path),
                source: ShaderSource::Wgsl(s.into()),
            };
            device.create_shader_module(desc)
        };
        let fs = {
            let mut f = File::open(fs_path)?;
            let mut s = String::new();
            f.read_to_string(&mut s);
            let desc = ShaderModuleDescriptor {
                label: Some(fs_path),
                source: ShaderSource::Wgsl(s.into()),
            };
            device.create_shader_module(desc)
        };
        // 这里可以放多个吗?
        let targets = [Some(wgpu::ColorTargetState {
            format: self.target_format.ok_or(anyhow!(BUILDER_FIELD_UNSET))?,
            blend: Some(self.target_blend.ok_or(anyhow!(BUILDER_FIELD_UNSET))?),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipline = {
            let desc = RenderPipelineDescriptor {
                label: label.as_deref(),
                layout: Some(layout),
                vertex: VertexState {
                    module: &vs,
                    entry_point: Self::VS_FUNC_NAME,
                    buffers: vertex_buffer,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fs,
                    entry_point: Self::FS_FUNC_NAME,
                    targets: &targets,
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    polygon_mode: PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: depth_format.clone(),
                    depth_write_enabled: depth_write.clone(),
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            };
            device.create_render_pipeline(&desc)
        };
        Ok(pipline)
    }
}

#[derive(Debug, Default)]
pub struct BindGroupBuider<'a> {
    device: Option<&'a Device>,
    label: Option<&'a str>,
    entries: Vec<BindGroupBuilderEntryDesc<'a>>,
}

#[derive(Debug)]
pub struct BindGroupBuilderEntryDesc<'a> {
    pub resource: BindingResource<'a>,
    pub visibility: ShaderStages,
    pub count: Option<NonZeroU32>,
    pub ty: BindingType,
}

impl<'a> BindGroupBuider<'a> {
    builder_set_fn!(set_device, device, &'a Device);
    builder_set_fn!(set_label, label, &'a str);

    pub fn push_entry(&mut self, desc: BindGroupBuilderEntryDesc<'a>) {
        self.entries.push(desc);
    }
    pub fn push_entries<T>(&mut self, descs: T)
    where
        T: IntoIterator<Item = BindGroupBuilderEntryDesc<'a>>,
    {
        for desc in descs {
            self.push_entry(desc)
        }
    }

    pub fn build(mut self) -> Result<(BindGroupLayout, BindGroup)> {
        let device = self.device.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let entries = replace(&mut self.entries, Vec::new());
        let (layout_entries, entries): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let layout = BindGroupLayoutEntry {
                    binding: i as u32,
                    count: e.count,
                    visibility: e.visibility,
                    ty: e.ty,
                };
                let entry = BindGroupEntry {
                    binding: i as u32,
                    resource: e.resource,
                };
                (layout, entry)
            })
            .unzip();
        let layout = {
            let label = get_default_label(&self.label, [BIND_GROUP_LAYOUT_LABEL]);
            let desc = BindGroupLayoutDescriptor {
                label: label.as_deref(),
                entries: &layout_entries,
            };
            device.create_bind_group_layout(&desc)
        };
        let bind_group = {
            let label = get_default_label(&self.label, [BIND_GROUP_LABEL]);
            let desc = BindGroupDescriptor {
                label: label.as_deref(),
                layout: &layout,
                entries: &entries,
            };
            device.create_bind_group(&desc)
        };
        Ok((layout, bind_group))
    }
}

const BUILDER_FIELD_UNSET: &'static str = "builder 必须字段未被设置";
