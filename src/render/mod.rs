use std::{
    fs::File,
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

use crate::{utils::builder_set_fn, window::Input};

pub mod camera;
pub mod texture;
use camera::*;
use texture::*;

pub const BIND_GROUP_LAYOUT_LABEL: &'static str = " BindGroupLayout";
pub const BUFFER_LABEL: &'static str = " Buffer";

#[derive(Debug)]
pub struct RenderState {
    pub device: Device,
    pub queue: Queue,
    pub main_surface: Surface,
    pub main_surface_config: SurfaceConfiguration,
    pub cube_pipeline: RenderPipeline,

    pub camera: Camera,
    pub camera_bind: CameraBind,
    pub bind_groups: Vec<BindGroup>,
    pub clear_color: Color,
}

impl RenderState {
    pub async fn init(window: &Window) -> Result<RenderState> {
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
            let desc = DeviceDescriptor {
                label: None,
                features: Features::empty(),
                limits: Limits::default(),
            };
            adapter
                .request_device(&desc, None)
                .await
                .expect("request device 失败")
        };

        // 后面是渲染相关了

        let clear_color = Color::GREEN;

        // surface 配置, 窗口 resize 用
        let con = {
            let size = window.inner_size();
            let con =
                Self::get_default_main_surface_config(size.width, size.height, &surface, &adapter);
            surface.configure(&device, &con);
            con
        };

        // 存所有的 bind group, bind group layout
        let mut bind_group_layouts = Vec::new();
        let mut bind_groups = Vec::new();

        // bind group 开始 几乎每帧更新, 且通用
        let mut group_bd = BindGroupBuider::default();
        group_bd.set_device(&device).set_label("Per Frame");

        // 相机
        let mut camera = Camera::default();
        camera.calculate();

        // 相机 bind, 并增加到 bind group.
        let mut builder = CameraDescBuilder::default();
        builder
            .set_device(&device)
            .set_camera(&camera)
            .set_label("Main");
        let camera_bind = builder.build();
        let entries = camera_bind.get_entries_desc();
        group_bd.push_entries(entries);

        // bind group 结束
        let (lay, bg) = {
            group_bd.complete();
            group_bd.drain()
        };
        bind_group_layouts.push(lay);
        bind_groups.push(bg);

        // pipeline
        let layout = {
            let desc = PipelineLayoutDescriptor {
                label: Some("Cube Pipline Layout"),
                bind_group_layouts: &[&bind_group_layouts[0]],
                push_constant_ranges: &[],
            };
            device.create_pipeline_layout(&desc)
        };

        let pipeline = {
            let mut builder = PipelineBuilder::new();
            let vertex_buffer = [Vertex::desc()];
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

        todo!()
    }

    pub fn redraw(&mut self) -> anyhow::Result<()> {
        let texture = self.main_surface.get_current_texture()?;
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
                // let depth_stencil_att = RenderPassDepthStencilAttachment {
                //     view: todo!(),
                //     depth_ops: Some(Operations {
                //         load: LoadOp::Clear(1.0),
                //         store: true,
                //     }),
                //     stencil_ops: None,
                // };
                let desc = RenderPassDescriptor {
                    label: Some("主 Render Pass"),
                    color_attachments: &[Some(color_att)],
                    depth_stencil_attachment: None,
                };
                encoder.begin_render_pass(&desc)
            };

            render_pass.set_pipeline(&self.cube_pipeline)
        }
        let command_buffer = encoder.finish();
        self.queue.submit(once(command_buffer));
        texture.present();
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.main_surface_config.width = width;
        self.main_surface_config.height = height;
        self.main_surface
            .configure(&self.device, &self.main_surface_config);
    }
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

pub fn redraw(render: &mut RenderState, input: &Input) {}

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

        let device = replace(&mut self.device, None).unwrap();
        let layout = replace(&mut self.layout, None).unwrap();
        let vs_path = replace(&mut self.vs_path, None).unwrap();
        let fs_path = replace(&mut self.fs_path, None).unwrap();
        let label = self.label;
        let vertex_buffer = replace(&mut self.vertex_buffer, None).unwrap();
        let depth_format = replace(&mut self.depth_format, None).unwrap();
        let depth_write = replace(&mut self.depth_write, None).unwrap();

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
                source: ShaderSource::Wgsl(fs_path.into()),
            };
            device.create_shader_module(desc)
        };
        // 这里可以放多个吗?
        let targets = [Some(wgpu::ColorTargetState {
            format: self.target_format.unwrap(),
            blend: Some(self.target_blend.unwrap()),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipline = {
            let desc = RenderPipelineDescriptor {
                label: label,
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
                    format: depth_format,
                    depth_write_enabled: depth_write,
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

const TEST_VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0, 0.0],
        color: [0.5, 0.0, 0.5],
        tex_coords: [0.0, 0.0],
    }, // A
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.5, 0.0, 0.5],
        tex_coords: [0.0, 0.0],
    }, // B
    Vertex {
        position: [1.0, 1.0, 0.0],
        color: [0.5, 0.0, 0.5],
        tex_coords: [1.0, 0.0],
    }, // C
    Vertex {
        position: [1.0, 0.0, 0.0],
        color: [0.5, 0.0, 0.5],
        tex_coords: [1.0, 0.0],
    }, // D
];

const TEST_INDICES: &[u16] = &[0, 3, 2, 2, 1, 0];

#[repr(C)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        vertex_buffer_layout!(Vertex ; Vertex; struct {
            0;position ; Float32x3,
            1;color ; Float32x3,
            2;tex_coords ; Float32x2,
        })
    }

    // fn desc2<'a>() -> (VertexBufferLayout<'a>, ){
    //     let attrs = [
    //         wgpu::VertexAttribute {
    //             offset: 0,
    //             shader_location: 0,
    //             format: wgpu::VertexFormat::Float32x3,
    //         },
    //         wgpu::VertexAttribute {
    //             offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
    //             shader_location: 1,
    //             format: wgpu::VertexFormat::Float32x3,
    //         },
    //         wgpu::VertexAttribute {
    //             offset: std::mem::size_of::<([f32; 3], [f32; 3])>() as wgpu::BufferAddress,
    //             shader_location: 2,
    //             format: wgpu::VertexFormat::Float32x2,
    //         },
    //     ];
    //     let attrs = Rc::new(attrs);
    //     let layout = wgpu::VertexBufferLayout {
    //         array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Vertex,
    //         attributes: attrs.as_ref(),
    //     };
    //     return (layout,)

    // }
}

macro_rules! field_desc {
    ($t:ty ; struct { $( $sloc:expr; $field:ident ; $format:ident,)+ }) => {
        field_desc!(@INNER offset_of, $t {$($sloc; $field ; $format,)+});
    };
    ($t:ty ; tuple { $( $sloc:expr; $field:ident ; $format:ident,)+ }) => {
        field_desc!(@INNER offset_of_tuple, $t {$($sloc; $field ; $format,)+});
    };
    (@INNER $offset_fn:tt, $t:ty { $($sloc:expr; $field:ident ; $format:ident,)+ }) => {
        [$(
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::$format,
                offset: memoffset::$offset_fn!($t, $field) as wgpu::BufferAddress,
                shader_location: $sloc,
            }
        ),+]
    };
}

#[macro_export]
macro_rules! vertex_buffer_layout {
    ($t:ty; $step_mode:ident; struct { $( $sloc:expr; $field:ident ; $format:ident,)+ }) => {{
        static ATTRS: once_cell::sync::OnceCell<[wgpu::VertexAttribute; 3]> = once_cell::sync::OnceCell::new();
        let attrs = ATTRS.get_or_init(|| {
            field_desc!($t ; struct {$($sloc; $field ; $format,)+})
        });
        VertexBufferLayout {
            array_stride: std::mem::size_of::<$t>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::$step_mode,
            attributes: attrs,
        }
    }};
}

pub(self) use field_desc;
pub use vertex_buffer_layout;

#[derive(Debug, Default)]
pub struct BindGroupBuider<'a> {
    complete: bool,

    device: Option<&'a Device>,
    label: Option<&'a str>,
    entries: Vec<BindGroupBuilderEntryDesc<'a>>,

    layout: Option<BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,
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

    fn get_label_or_default(&self) -> Option<&str> {
        self.label.or_else(|| Some("Unnamed Bind Group"))
    }

    fn get_bind_group_layout_label(&mut self) -> Option<String> {
        let name = self.get_label_or_default()?;
        let mut name = name.to_string();
        name.push_str(BIND_GROUP_LAYOUT_LABEL);
        Some(name)
    }
    pub fn complete(&mut self) {
        if self.complete {
            panic!("只能 complete 一次")
        }
        self.complete = true; 
        
        if self.layout.is_some() || self.bind_group.is_some() {
            unreachable!()
        }

        let device = self.device.unwrap();
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
            let label = self.get_bind_group_layout_label();
            let desc = BindGroupLayoutDescriptor {
                label: label.as_deref(),
                entries: &layout_entries,
            };
            device.create_bind_group_layout(&desc)
        };
        let bind_group = {
            let label = self.get_label_or_default();
            let desc = BindGroupDescriptor {
                label: label,
                layout: &layout,
                entries: &entries,
            };
            device.create_bind_group(&desc)
        };

        self.layout = Some(layout);
        self.bind_group = Some(bind_group);
    }

    pub fn drain(self) -> (BindGroupLayout, BindGroup) {
        if !self.complete {
            panic!("需要先 complete")
        };
        (self.layout.unwrap(), self.bind_group.unwrap())
    }
}
