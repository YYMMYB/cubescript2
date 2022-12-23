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
use image::DynamicImage;
use memoffset::offset_of;
use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Projective3, Vector3};
use once_cell::sync::OnceCell;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::{scene::Scene, utils::builder_set_fn};

pub mod built_in;
pub mod camera;
pub mod label;
pub mod mesh;
pub mod pipeline;
pub mod texture;
use built_in::*;
use camera::*;
use label::*;
use mesh::*;
use pipeline::*;
use texture::*;

#[derive(Debug)]
pub struct RenderState {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub surface_config: SurfaceConfiguration,

    pub bind_groups: Vec<BindGroup>,
    pub camera_bind: CameraBind,
    pub depth_texture_bind: TextureBind,

    pub cube_pipeline: cube::Pipeline,

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

        // 相机 bind, 并增加到 bind group.
        let mut camera_bind = camera.create_binding(&device);
        let camera_res = camera_bind.get_bind_resource();
        let lay = create_bind_group_layout(
            &device,
            Some("Camera Bind Group Layout"),
            &Camera::get_layout_args(),
        )?;
        let bg = create_bind_group(&device, Some("Camera Bind Group"), &lay, &camera_res)?;
        bind_group_layouts.push(lay);
        bind_groups.push(bg);

        // 深度图
        let mut desc = TextureArgs::depth_texture();
        desc.width = surface_config.width;
        desc.height = surface_config.height;

        let depth_format = desc.format;
        let depth_texture_bind = {
            let texture = device.create_texture(&desc.into_desc(Some("Depth Texture")));
            let view = texture.create_view(&TextureViewDescriptor::default());
            TextureBind { texture, view }
        };

        // cube 管线
        let cube_pipeline = cube::PipelinePreparer::init()?.create_pipeline(
            &device,
            &queue,
            &bind_group_layouts,
            surface_config.format,
            depth_format,
        )?;

        let mut ret = Self {
            device,
            queue,
            surface,
            surface_config,
            cube_pipeline,
            camera_bind,
            bind_groups,
            depth_texture_bind,
            mesh_manager,
        };
        Ok(ret)
    }

    pub fn redraw(&mut self, camera: &Camera, scene: &mut Scene) -> anyhow::Result<()> {
        self.camera_bind.write(&mut self.queue, camera);

        let texture = self.surface.get_current_texture()?;
        let mut encoder = {
            let desc = CommandEncoderDescriptor {
                label: Some("主要 Command Encoder"),
            };
            self.device.create_command_encoder(&desc)
        };
        let main_surface_view = texture.texture.create_view(&Default::default());

        {
            let mut rp = self.cube_pipeline.start_pass(
                &mut encoder,
                &main_surface_view,
                &self.depth_texture_bind.view,
                &self.bind_groups,
            );
            self.cube_pipeline
                .draw(&self.queue, &mut rp, &mut scene.cubes)?;
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
        let mut desc = TextureArgs::depth_texture();
        desc.width = self.surface_config.width;
        desc.height = self.surface_config.height;
        let tx = self
            .device
            .create_texture(&desc.into_desc(Some("Depth Texture (resized)")));
        self.depth_texture_bind = TextureBind {
            view: tx.create_view(&TextureViewDescriptor::default()),
            texture: tx,
        };
        Ok(())
    }
}

pub fn ttt<'a, 'b>(p: &'b cube::Pipeline, rp: &'b mut RenderPass<'a>) {}

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
