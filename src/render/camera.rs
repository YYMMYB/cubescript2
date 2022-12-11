use std::mem::{replace, size_of};

use nalgebra::*;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::{utils::builder_set_fn, window::Input};

use super::*;

#[repr(C)]
#[derive(Debug)]
pub struct Camera {
    view_matrix: [[f32; 4]; 4],
    proj_matrix: [[f32; 4]; 4],

    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,

    pub position: Point3<f32>,
    pub direction: Vector3<f32>,
    pub up: Vector3<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            view_matrix: Default::default(),
            proj_matrix: Default::default(),

            position: Point3::origin(),
            direction: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),

            aspect: 16.0 / 9.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }
}

impl Camera {
    pub fn calculate_view(&mut self) {
        let target = self.position + self.direction;
        let view = Isometry3::look_at_rh(&self.position, &target, &self.up);
        self.view_matrix = view.to_matrix().into();
    }

    pub fn calculate_proj(&mut self) {
        let proj = Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar);
        #[rustfmt::skip]
        let OPENGL_TO_WGPU_MATRIX = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.5,
            0.0, 0.0, 0.0, 1.0,
        );
        self.proj_matrix = (OPENGL_TO_WGPU_MATRIX * proj.into_inner()).into();
    }

    pub fn calculate(&mut self) {
        self.calculate_view();
        self.calculate_proj();
    }
}

#[derive(Debug)]
pub struct CameraBind {
    pub view_buffer: Buffer,
    pub proj_buffer: Buffer,
}

impl CameraBind {
    pub fn write(&self, queue: &mut Queue, camera: &Camera) {
        queue.write_buffer(
            &self.view_buffer,
            0,
            bytemuck::cast_slice(&camera.view_matrix),
        );
        queue.write_buffer(
            &self.proj_buffer,
            0,
            bytemuck::cast_slice(&camera.proj_matrix),
        );
    }

    pub fn get_entries_desc(
        &self,
    ) -> [BindGroupBuilderEntryDesc; 2] {
        let view_buffer = &self.view_buffer;
        let view_desc = BindGroupBuilderEntryDesc {
            resource: view_buffer.as_entire_binding(),
            visibility: wgpu::ShaderStages::VERTEX,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };

        let proj_buffer = &self.proj_buffer;
        let proj_desc = BindGroupBuilderEntryDesc {
            resource: proj_buffer.as_entire_binding(),
            visibility: wgpu::ShaderStages::VERTEX,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        [view_desc, proj_desc]
    }
}

#[derive(Default, Debug)]
pub struct CameraDescBuilder<'a> {
    complete: bool,

    camera: Option<&'a Camera>,
    device: Option<&'a Device>,
    label: Option<&'a str>,

    view_buffer_label: Option<String>,
    proj_buffer_label: Option<String>,

    view_buffer: Option<Buffer>,
    proj_buffer: Option<Buffer>,
}

impl<'d> CameraDescBuilder<'d> {
    pub const VIEW_LABEL: &'static str = " View Matrix4";
    pub const PROJ_LABEL: &'static str = " Proj Matrix4";

    builder_set_fn!(set_camera, camera, &'d Camera);
    builder_set_fn!(set_device, device, &'d Device);
    builder_set_fn!(set_label, label, &'d str);

    fn get_label_or_default(&self) -> Option<&str> {
        self.label.or_else(|| Some("Unnamed Camera"))
    }

    fn complete_buffer_label(&mut self) {
        if self.view_buffer_label.is_none() {
            let Some(name) = self.get_label_or_default() else {return;};
            let mut name = name.to_string();
            name.push_str(Self::VIEW_LABEL);
            name.push_str(BUFFER_LABEL);
            self.view_buffer_label = Some(name);
        }
        if self.proj_buffer_label.is_none() {
            let Some(name) = self.get_label_or_default() else {return;};
            let mut name = name.to_string();
            name.push_str(Self::PROJ_LABEL);
            name.push_str(BUFFER_LABEL);
            self.proj_buffer_label = Some(name);
        }
    }

    pub fn build(mut self) -> CameraBind {
        let device = self.device.unwrap();
        let camera = self.camera.unwrap();

        self.complete_buffer_label();

        let view_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: self.view_buffer_label.as_deref(),
            contents: bytemuck::cast_slice(&camera.view_matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let proj_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: self.proj_buffer_label.as_deref(),
            contents: bytemuck::cast_slice(&camera.proj_matrix),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        CameraBind {
            view_buffer,
            proj_buffer,
        }
    }
}
