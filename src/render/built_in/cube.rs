use std::{
    fs::File,
    hash::Hash,
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::{NonZeroI64, NonZeroU32, NonZeroU64},
    rc::Rc,
};

use anyhow::Result;
use bytemuck::cast_slice;
use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgba};
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
    render::texture::format_info::TextureFormatPixelInfo,
    utils::*,
};
use resource::*;

use super::super::*;

// +x 面, 需要与 id 为 000000 的方向保持一直, orient 才能合理的表示方向.
pub const TEST_VERTICES: &[CubeVertx] = &[
    CubeVertx {
        position: [1.0, 1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    CubeVertx {
        position: [1.0, -1.0, 1.0],
        tex_coords: [0.0, 1.0],
    },
    CubeVertx {
        position: [1.0, -1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    CubeVertx {
        position: [1.0, 1.0, -1.0],
        tex_coords: [1.0, 0.0],
    },
];

pub const TEST_INDICES: &[u16] = &[2, 3, 0, 0, 1, 2];

pub const TEST_INSTANCES: &[CubeInstance] = &[
    // // 参考 黑
    // CubeInstance {
    //     info: [0, 0b000000, 0, 0],
    //     position: [0.0, 0.0, -4.0],
    //     color: [1.0, 1.0, 1.0],
    // },
    // -x 红
    CubeInstance {
        info: [0, 0b001000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [1.0, 0.01, 0.01],
    },
    // +x 红
    CubeInstance {
        info: [0, 0b000000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [1.0, 0.01, 0.01],
    },
    // -y 绿
    CubeInstance {
        info: [0, 0b011000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [0.01, 1.0, 0.01],
    },
    // +y 绿
    CubeInstance {
        info: [0, 0b010000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [0.01, 1.0, 0.01],
    },
    // -z 蓝
    CubeInstance {
        info: [0, 0b101000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [0.01, 0.01, 1.0],
    },
    // +z 蓝
    CubeInstance {
        info: [0, 0b100000, 0, 0],
        position: [0.0, 0.0, 0.0],
        color: [0.01, 0.01, 1.0],
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
    pub info: [u8; 4], // [指数(2的几次方, 缩放用), 旋转id(0..48), 贴图id与偏移0(低位), 贴图id与偏移1(高位)]
    pub position: [f32; 3], // 先做info里的rotation_id, 再做这里的position
    pub color: [f32; 3],
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

const ORIENT_COUNT: usize = 24;
type MATRIX = [[f32; 4]; 4];
#[derive(Debug)]
pub struct ConstResource {
    pub rot_mat: [MATRIX; ORIENT_COUNT],
    pub paths: Vec<String>,
}

impl ConstResource {
    pub fn init() -> Self {
        let rot: MATRIX = Default::default();
        let mut rot_mat: [MATRIX; ORIENT_COUNT] = [rot; ORIENT_COUNT];
        for code in 0..ORIENT_COUNT as u8 {
            let orint = Orient::<CompressedData>::decode(code << 1).uncompress();
            let mat = orint.to_matrix_without_flip();
            rot_mat[code as usize] = mat.to_homogeneous().into();
        }
        let paths = vec![
            "image/cube_test".to_string(),
            "image/cube_test_2".to_string(),
        ];
        Self { rot_mat, paths }
    }
    pub fn create_bind(&self, device: &Device, queue: &Queue) -> Result<ConstResourceBind> {
        let rot_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Cube Resource Rot Matrix"),
            contents: cast_slice(&self.rot_mat),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let format = TextureFormat::Rgba8UnormSrgb;
        let len = self.paths.len();
        let texture_array = {
            let mut image_array = Vec::new();
            let is_srgb = true;
            for path in self.paths.iter() {
                let img = ImageMipMap::from_path(path, is_srgb)?;
                image_array.push(img);
            }
            let mut desc = TextureArgs::texture_array();
            desc.format = if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };
            desc.width = image_array[0].width;
            desc.height = image_array[0].height;
            desc.depth = len as u32;
            desc.mip_level_count = image_array[0].mip_map_count();
            let tx = device.create_texture(&desc.into_desc(Some("CubeTex")));
            for i in 0..len {
                image_array[i].write_into_texture(queue, &tx, i as u32);
            }
            tx
        };

        let sampler = {
            let desc = SamplerDescriptor {
                label: Some("CubeTexSampler"),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                ..Default::default()
            };
            device.create_sampler(&desc)
        };

        let view = {
            let desc = TextureViewDescriptor {
                label: Some("CubeTexView"),
                dimension: Some(TextureViewDimension::D2Array),
                // base_array_layer: 0,
                // array_layer_count: Some(NonZeroU32::new(len as u32).unwrap()),
                ..Default::default()
            };
            texture_array.create_view(&desc)
        };

        Ok(ConstResourceBind {
            rot_mat: rot_buffer,
            texture: texture_array,
            array_view: view,
            sampler: sampler,
        })
    }

    pub fn get_layout_args() -> Result<[BindGroupLayoutEntryArgs; 3]> {
        let rot_mat = BindGroupLayoutEntryArgs {
            count: None,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        let sampler = BindGroupLayoutEntryArgs {
            count: None,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
        };
        let texture = BindGroupLayoutEntryArgs {
            count: None,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
        };
        Ok([rot_mat, sampler, texture])
    }
}

#[derive(Debug)]
pub struct ConstResourceBind {
    pub rot_mat: Buffer,
    pub texture: Texture,
    pub array_view: TextureView,
    pub sampler: Sampler,
}

impl ConstResourceBind {
    pub fn get_bind_resource(&self) -> Result<[BindingResource; 3]> {
        Ok([
            self.rot_mat.as_entire_binding(),
            BindingResource::Sampler(&self.sampler),
            BindingResource::TextureView(&self.array_view),
        ])
    }
    pub fn get_entries_desc<'a>(&'a self) -> [BindGroupBuilderEntryDesc<'a>; 3] {
        let rot_desc = BindGroupBuilderEntryDesc {
            resource: self.rot_mat.as_entire_binding(),
            visibility: ShaderStages::VERTEX,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        let tex_desc = {
            let ret = BindGroupBuilderEntryDesc {
                resource: BindingResource::TextureView(&self.array_view),
                count: None,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
            };
            ret
        };
        let sampler_desc = BindGroupBuilderEntryDesc {
            resource: BindingResource::Sampler(&self.sampler),
            visibility: ShaderStages::FRAGMENT,
            count: None,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
        };
        [rot_desc, sampler_desc, tex_desc]
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

pub struct PipelinePreparer {
    pub vs: Shader,
    pub fs: Shader,
}

impl PipelinePreparer {
    pub fn init() -> Result<Self> {
        let vs = Shader::from_path(get_abs_path(VS_PATH)?, ShaderType::Wgsl, Shader::VS_FUNC_NAME.to_string())?;
        let fs = Shader::from_path(get_abs_path(FS_PATH)?, ShaderType::Wgsl, Shader::FS_FUNC_NAME.to_string())?;
        Ok(Self { vs, fs })
    }

    pub fn create_pipeline<'a, I>(
        &'a self,
        device: &'a Device,
        queue: &'a Queue,
        group_layouts: I,
        target_format: TextureFormat,
        depth_format: TextureFormat,
    ) -> Result<Pipeline>
    where
        I: IntoIterator<Item = &'a BindGroupLayout>,
    {
        // 绑定组
        let const_layout = create_bind_group_layout(
            device,
            Some("Cube Const Resource Group Layout"),
            &ConstResource::get_layout_args()?,
        )?;
        let const_group = {
            let const_bind = {
                let res = ConstResource::init();
                res.create_bind(device, queue)?
            };
            let binding = const_bind.get_bind_resource()?;
            create_bind_group(device, Some("Const Group"), &const_layout, &binding)?
        };

        // 创建管线
        let extend = [&const_layout];
        let pipe_layout = {
            let group_layouts = group_layouts.into_iter().map(|l| l).chain(extend);
            create_pipeline_layout(device, Some("Cube Pipeline Layout"), group_layouts)?
        };
        let vs = create_shader_module(device, Some("Cube VS"), self.vs.data.as_shader_source())?;
        let fs = create_shader_module(device, Some("Cube FS"), self.fs.data.as_shader_source())?;
        let v = CubeVertx::attr_desc();
        let i = CubeInstance::attr_desc();
        let vbl = {
            let v = CubeVertx::desc(&v);
            let i = CubeInstance::desc(&i);
            [v, i]
        };
        // 不透明的管线, 后面透明管线应该会有很多重复的参数, 再想办法
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Cube Pipeline"),
            layout: Some(&pipe_layout),
            vertex: VertexState {
                module: &vs,
                entry_point: self.vs.enter_point(),
                buffers: &vbl,
            },
            fragment: Some(FragmentState {
                module: &fs,
                entry_point: self.fs.enter_point(),
                targets: &[Some(ColorTargetState {
                    format: target_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
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
                depth_write_enabled: true,
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
        });

        // 其他东西

        // 共用的 vertex 和 index
        let vertex = create_buffer(
            device,
            Some("Cube Vertex"),
            BufferUsages::VERTEX,
            TEST_VERTICES,
        );
        let index = create_buffer(
            device,
            Some("Cube Index"),
            BufferUsages::INDEX,
            TEST_INDICES,
        );

        Ok(Pipeline {
            pipeline,
            groups: vec![const_group],
            vertex,
            index,
            index_len: TEST_INDICES.len() as u32,
        })
    }
}

#[derive(Debug)]
pub struct Pipeline {
    pub pipeline: RenderPipeline,
    pub groups: Vec<BindGroup>,
    pub vertex: Buffer,
    pub index: Buffer,
    pub index_len: u32,
}
impl Pipeline {
    // 一般只调用一次
    pub fn start_pass<'a>(
        &'a self,
        encoder: &'a mut CommandEncoder,
        target_view: &'a TextureView,
        depth_view: &'a TextureView,
        global_groups: impl IntoIterator<Item = &'a BindGroup>,
    ) -> RenderPass<'a> {
        // 创建管线
        let mut render_pass = {
            let color_att = RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            };
            let depth_stencil_att = RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            };
            let desc = RenderPassDescriptor {
                label: Some("Cube Opaque Render Pass"),
                color_attachments: &[Some(color_att)],
                depth_stencil_attachment: Some(depth_stencil_att),
            };
            encoder.begin_render_pass(&desc)
        };

        // 通用资源绑定
        render_pass.set_pipeline(&self.pipeline);
        let mut idx = 0;
        for g in global_groups {
            render_pass.set_bind_group(idx, g, &[]);
            idx += 1;
        }
        for g in &self.groups {
            render_pass.set_bind_group(idx, g, &[]);
            idx += 1;
        }
        render_pass.set_vertex_buffer(0, self.vertex.slice(..));
        render_pass.set_index_buffer(self.index.slice(..), IndexFormat::Uint16);
        render_pass
    }

    // 可多次调用
    pub fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut RenderPass<'a>,
        instance_buffer: &'a Buffer,
        instance_len: u32,
    ) {
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.draw_indexed(0..self.index_len, 0, 0..instance_len);
    }
}

const VS_PATH: &'static str = "shader/cube_shader.wgsl";
const FS_PATH: &'static str = "shader/cube_shader.wgsl";
