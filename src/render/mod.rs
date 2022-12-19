use std::{
    collections::HashSet,
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

use crate::utils::builder_set_fn;

pub mod built_in;
pub mod camera;
pub mod label;
pub mod mesh;
pub mod pipeline;
pub mod scene;
pub mod texture;
use built_in::*;
use camera::*;
use label::*;
use mesh::*;
use pipeline::*;
use scene::*;
use texture::*;

#[derive(Debug)]
pub struct RenderState {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,

    pub camera_bind: CameraBind,
    pub bind_groups: Vec<BindGroup>,
    pub clear_color: Color,
    pub depth_texture_bind: TextureBind,

    pub cube_const_resource: cube::ConstResource,
    pub cube_const_resource_bind: cube::ConstResourceBind,
    pub cube_pipeline: RenderPipeline,

    pub mesh_manager: MeshManager,
}

impl RenderState {
    pub async fn init(window: &Window, camera: &Camera) -> Result<RenderState> {
        // 创建 surface(交换链)
        let instance = Instance::new(Backends::all());
        let surface = unsafe { instance.create_surface(window) };

        // 创建 device , queue
        let adapter = {
            let opt = RequestAdapterOptions {
                power_preference: Default::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            };
            instance
                .request_adapter(&opt)
                .await
                .expect("request adapter 失败")
        };

        let (device, queue) = {
            let mut limits = Limits::default();
            limits.min_uniform_buffer_offset_alignment = 64;
            let desc = DeviceDescriptor {
                label: None,
                features: Features::BUFFER_BINDING_ARRAY,
                limits: limits,
            };
            adapter
                .request_device(&desc, None)
                .await
                .expect("request device 失败")
        };

        // 后面是渲染相关了

        let clear_color = Color::GREEN;

        // surface 配置, 窗口 resize 用
        let surface_config = {
            let size = window.inner_size();
            let con =
                Self::get_default_main_surface_config(size.width, size.height, &surface, &adapter);
            surface.configure(&device, &con);
            con
        };

        // Mesh Manager
        let mesh_manager = MeshManager::init(&device)?;

        // 存所有的 bind group, bind group layout
        let mut bind_group_layouts = Vec::new();
        let mut bind_groups = Vec::new();

        // bind group 开始 几乎每帧更新, 且通用
        let mut group_bd = BindGroupBuider::default();
        group_bd.set_device(&device).set_label("Per Frame");

        // 相机 bind, 并增加到 bind group.
        let mut builder = CameraBindBuilder::default();
        builder
            .set_device(&device)
            .set_camera(&camera)
            .set_label("Main");
        let camera_bind = builder.build()?;
        let camera_entries = camera_bind.get_entries_desc();

        // bind group 结束
        group_bd.push_entries(camera_entries);
        let (lay, bg) = group_bd.build()?;
        bind_group_layouts.push(lay);
        bind_groups.push(bg);

        // bind group 开始 cube
        let cube_const_resource = cube::ConstResource::init();
        let cube_const_resource_bind = cube_const_resource.create_bind(&device, &queue)?;
        // cube_const_resource_bind.write(&queue, &cube_const_resource);
        let (lay, bg) = cube::build_bind_group(&device, &cube_const_resource_bind)?;
        // bind group 结束
        bind_group_layouts.push(lay);
        bind_groups.push(bg);

        // pipeline
        let layout = {
            let desc = PipelineLayoutDescriptor {
                label: Some("Cube Pipline Layout"),
                bind_group_layouts: &[&bind_group_layouts[0], &bind_group_layouts[1]],
                // bind_group_layouts: &[&bind_group_layouts[0]],
                // todo 相机vp, dt每帧更新的都换成 push_constants
                push_constant_ranges: &[],
            };
            device.create_pipeline_layout(&desc)
        };

        let cube_pipeline = {
            let mut builder = PipelineBuilder::new();
            let (v_lay, i_lay) = {
                let vid = &mesh_manager.cube_meshs[0].vertex_layout_id;
                let iid = &mesh_manager.cube_meshs[0].instance_layout_id;
                let v = mesh_manager
                    .attr_lay_set
                    .get(vid)
                    .ok_or(anyhow!(EMPTY_KEY))?;
                let i = mesh_manager
                    .attr_lay_set
                    .get(iid)
                    .ok_or(anyhow!(EMPTY_KEY))?;
                (v, i)
            };
            let vertex_buffer = [
                cube::CubeVertx::desc(v_lay),
                cube::CubeInstance::desc(i_lay),
            ];
            builder
                .set_device(&device)
                .set_label("Cube Pipline")
                .set_layout(&layout)
                .set_target_blend(BlendState::REPLACE)
                .set_target_format(TextureFormat::Bgra8UnormSrgb)
                .set_vs_path("shader/cube_shader.wgsl")
                .set_fs_path("shader/cube_shader.wgsl")
                .set_vertex_buffer(&vertex_buffer);
            builder.build()?
        };

        // 深度图
        let depth_texture_bind = create_depth_texture_bind(&device, &surface_config)?;

        let mut ret = Self {
            device,
            queue,
            surface,
            surface_config,
            cube_pipeline,
            camera_bind,
            bind_groups,
            clear_color,
            depth_texture_bind,
            mesh_manager,
            cube_const_resource,
            cube_const_resource_bind,
        };
        Ok(ret)
    }

    pub fn redraw(&mut self, camera: &Camera) -> anyhow::Result<()> {
        self.camera_bind.write(&mut self.queue, camera);

        let texture = self.surface.get_current_texture()?;
        let mut encoder = {
            let desc = CommandEncoderDescriptor {
                label: Some("主要 Command Encoder"),
            };
            self.device.create_command_encoder(&desc)
        };
        {
            let main_surface_view = texture.texture.create_view(&Default::default());
            let mut render_pass = {
                let color_att = RenderPassColorAttachment {
                    view: &main_surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                };
                let depth_stencil_att = RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_bind.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                };
                let desc = RenderPassDescriptor {
                    label: Some("主 Render Pass"),
                    color_attachments: &[Some(color_att)],
                    depth_stencil_attachment: Some(depth_stencil_att),
                };
                encoder.begin_render_pass(&desc)
            };

            render_pass.set_pipeline(&self.cube_pipeline);
            render_pass.set_bind_group(0, &self.bind_groups[0], &[]);
            render_pass.set_bind_group(1, &self.bind_groups[1], &[]);

            let cube_bind = &self.mesh_manager.cube_mesh_binds[0];
            let cube_mesh = &self.mesh_manager.cube_meshs[0];
            render_pass.set_vertex_buffer(0, cube_bind.vertex_bind.slice(..));
            render_pass.set_index_buffer(cube_bind.index_bind.slice(..), cube_mesh.index_format);
            for i in 0..self.mesh_manager.cube_mesh_binds.len() {
                let cube_bind = &self.mesh_manager.cube_mesh_binds[i];
                let cube_mesh = &self.mesh_manager.cube_meshs[i];
                let index_len = cube_mesh.indices.len() as u32;
                let mut instance_len = cube_mesh.instance.len() as u32;
                if instance_len < 1 {
                    instance_len = 1;
                }
                render_pass.set_vertex_buffer(1, cube_bind.instance_bind.slice(..));
                render_pass.draw_indexed(0..index_len, 0, 0..instance_len);
            }
        }
        let command_buffer = encoder.finish();
        self.queue.submit(once(command_buffer));
        texture.present();
        Ok(())
    }

    pub fn resize(&mut self, camera: &mut Camera, width: u32, height: u32) -> Result<()> {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        camera.aspect = width as f32 / height as f32;
        self.depth_texture_bind = create_depth_texture_bind(&self.device, &self.surface_config)?;
        Ok(())
    }
}

fn create_depth_texture_bind(
    device: &Device,
    surface_config: &SurfaceConfiguration,
) -> Result<TextureBind> {
    let mut builder = TextureBindBuilder::default();
    builder
        .set_device(device)
        .set_label("Depth")
        .set_format(TextureFormat::Depth32Float)
        .set_width(surface_config.width)
        .set_height(surface_config.height)
        .set_usage(TextureUsages::RENDER_ATTACHMENT);
    builder.build()
}

impl RenderState {
    fn get_default_main_surface_config(
        width: u32,
        height: u32,
        surface: &Surface,
        adapter: &Adapter,
    ) -> SurfaceConfiguration {
        let con = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(adapter)[0],
            width: width,
            height: height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
        };
        con
    }
}

const EMPTY_KEY: &'static str = "Key 不存在于字典中";
