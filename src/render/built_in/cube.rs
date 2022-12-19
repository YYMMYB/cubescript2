use std::{
    fs::File,
    hash::Hash,
    io::Read,
    iter::once,
    mem::{replace, size_of},
    num::{NonZeroU32, NonZeroU64, NonZeroI64},
    rc::Rc,
};

use anyhow::Result;
use bytemuck::cast_slice;
use cubescript2_macros::derive_desc;
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
    utils::*,
};

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
            let orint = Orient::<CompressedData>::decode(code<<1).uncompress();
            let mat = orint.to_matrix_without_flip();
            rot_mat[code as usize] = mat.to_homogeneous().into();
        }
        let paths = vec!["image/cube_test.png".to_string(),"image/cube_test_2.png".to_string()];
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
            let mip_len = 4;
            let size = Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: len as u32,
            };
            let desc = TextureDescriptor {
                label: Some("CubeTex"),
                size,
                mip_level_count: mip_len,
                sample_count: 1u32,
                dimension: TextureDimension::D2,
                format: format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            };
            let tx = device.create_texture(&desc);

            for i in 0..len {
                println!("{}", self.paths[i]);
                let img = image::open(&self.paths[i])?;
                let mut mips = vec![img.to_rgba8()];
                for mip_i in 1..mip_len {
                    let width = size.width >> mip_i;
                    let height = size.height >> mip_i;
                    let mut mip = ImageBuffer::new(width, height);
                    for x in 0..width {
                        for y in 0..height {
                            let mut sum = Rgba([0u8; 4]);
                            for dx in 0..2 {
                                for dy in 0..2 {
                                    let p = mips[mip_i as usize - 1].get_pixel(x * 2+dx, y * 2+dy);
                                    for c in 0..4 {
                                        sum.0[c] += p[c]/4;
                                    }
                                }
                            }
                            mip.put_pixel(x, y, sum);
                        }
                    }
                    // todo 测试代码, 记得删掉
                    // {
                    //     let mut name = String::new();
                    //     name.push_str("img_");
                    //     name.push_str(i.to_string().as_str());
                    //     name.push_str("_mip_");
                    //     name.push_str(mip_i.to_string().as_str());
                    //     name.push_str(".png");
                    //     mip.save_with_format(name,image::ImageFormat::Png)?;
                    // }
                    mips.push(mip);
                }

                let mips: Vec<_> = mips.into_iter().map(|mip|{
                    mip.into_raw()
                }).collect();

                for mip_i in 0..mip_len {
                    let width = size.width >> mip_i;
                    let height = size.height >> mip_i;
                    let t = ImageCopyTexture {
                        texture: &tx,
                        mip_level: mip_i,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: i as u32,
                        },
                        aspect: TextureAspect::All,
                    };
                    let data_lay = ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            NonZeroU32::new(format.pixel_size() as u32 * width).unwrap(),
                        ),
                        rows_per_image: None,
                    };
                    queue.write_texture(
                        t,
                        &mips[mip_i as usize][..],
                        data_lay,
                        Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
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
}

#[derive(Debug)]
pub struct ConstResourceBind {
    pub rot_mat: Buffer,
    pub texture: Texture,
    pub array_view: TextureView,
    pub sampler: Sampler,
}

impl ConstResourceBind {
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
