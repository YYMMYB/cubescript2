use std::mem::{replace, size_of};

use anyhow::*;
use nalgebra::*;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::window::Window;

use crate::{utils::builder_set_fn};

use super::*;

#[repr(C)]
#[derive(Debug)]
pub struct Camera {
    pub view_matrix: Matrix4<f32>,
    pub proj_matrix: Matrix4<f32>,

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
        let v:[[f32;4];4]= camera.view_matrix.clone().into();
        let p:[[f32;4];4]= camera.proj_matrix.clone().into();
        queue.write_buffer(
            &self.view_buffer,
            0,
            bytemuck::cast_slice(&v),
        );
        queue.write_buffer(
            &self.proj_buffer,
            0,
            bytemuck::cast_slice(&p),
        );
    }

    pub fn get_entries_desc(&self) -> [BindGroupBuilderEntryDesc; 2] {
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
pub struct CameraBindBuilder<'a> {
    camera: Option<&'a Camera>,
    device: Option<&'a Device>,
    label: Option<&'a str>,
}

impl<'d> CameraBindBuilder<'d> {
    pub const VIEW_LABEL: &'static str = " View Matrix4";
    pub const PROJ_LABEL: &'static str = " Proj Matrix4";

    builder_set_fn!(set_camera, camera, &'d Camera);
    builder_set_fn!(set_device, device, &'d Device);
    builder_set_fn!(set_label, label, &'d str);

    pub fn build(mut self) -> Result<CameraBind> {
        let device = self.device.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;
        let camera = self.camera.ok_or(anyhow!(BUILDER_FIELD_UNSET))?;

        let view_buffer_label = get_default_label(&self.label, [Self::VIEW_LABEL, BUFFER_LABEL]);
        let proj_buffer_label = get_default_label(&self.label, [Self::PROJ_LABEL, BUFFER_LABEL]);

        let v:[[f32;4];4]= camera.view_matrix.clone().into();
        let p:[[f32;4];4]= camera.proj_matrix.clone().into();
        let view_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: view_buffer_label.as_deref(),
            contents: bytemuck::cast_slice(&v),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let proj_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: proj_buffer_label.as_deref(),
            contents: bytemuck::cast_slice(&p),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(CameraBind {
            view_buffer,
            proj_buffer,
        })
    }
}

const BUILDER_FIELD_UNSET: &'static str = "builder 必须字段未被设置";
