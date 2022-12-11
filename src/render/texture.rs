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

use super::*;

pub struct Image {
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}
impl Image {
    pub fn from_image(path: &str, srgb: bool) -> Result<Self> {
        let img = image::open(path)?;
        let data = img.to_rgba8();
        let data = data.into_raw();
        Ok(Image {
            width: img.width(),
            height: img.height(),
            format: if srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
            data: data,
        })
    }
}

pub struct TextureBind {
    pub texture: wgpu::Texture,
    pub view: TextureView,
}

impl TextureBind {
    pub fn write(&self, queue: &Queue, image: &Image) {
        let data = &image.data;
        let size = Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        };
        let pixel_size = image.format.pixel_size();
        let data_layout = ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(NonZeroU32::new(pixel_size as u32 * size.width).unwrap()),
            rows_per_image: Some(NonZeroU32::new(size.height).unwrap()),
        };
        let texture = ImageCopyTexture {
            texture: &self.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        };
        queue.write_texture(texture, &data[..], data_layout, size)
    }

    fn get_entries_desc(&mut self) -> [BindGroupBuilderEntryDesc<'_>; 1] {
        let texture_desc = BindGroupBuilderEntryDesc {
            resource: BindingResource::TextureView(&self.view),
            visibility: ShaderStages::FRAGMENT,
            count: None,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
        };
        [texture_desc]
    }
}

pub struct TextureDescBuilder<'a> {
    device: Option<&'a Device>,
    label: Option<&'a str>,
    width: Option<u32>,
    height: Option<u32>,
    z: Option<u32>,
    format: Option<TextureFormat>,
    usage: Option<TextureUsages>,
}

const TEXTURE_LABEL: &'static str = " Texture";
const TEXTURE_VIEW_LABEL: &'static str = " Texture View";
const SAMPLER_LABEL: &'static str = " Sampler";

impl<'a> TextureDescBuilder<'a> {
    builder_set_fn!(set_device,device, &'a Device);
    builder_set_fn!(set_label,label, &'a str);
    builder_set_fn!(set_width,width, u32);
    builder_set_fn!(set_height,height, u32);
    builder_set_fn!(set_z,z, u32);
    builder_set_fn!(set_format,format, TextureFormat);
    builder_set_fn!(set_usage,usage, TextureUsages);

    pub fn set_size_by_image(&mut self, image: &Image) -> &mut Self {
        self.width = Some(image.width);
        self.height = Some(image.height);
        self.height = Some(1);

        self
    }

    fn get_label_or_default(&self) -> Option<&'a str> {
        self.label.or_else(|| Some("Unnamed"))
    }

    pub fn build(mut self) -> TextureBind {
        let device = self.device.unwrap();
        let label = self.get_label_or_default();

        let size = Extent3d {
            width: self.width.unwrap(),
            height: self.height.unwrap(),
            depth_or_array_layers: self.z.unwrap(),
        };

        let texture = {
            let label = label.map(|s| {
                let mut s = s.to_string();
                s.push_str(TEXTURE_LABEL);
                s
            });
            let desc = TextureDescriptor {
                label: label.as_deref(),
                size: size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format.take().unwrap(),
                usage: self.usage.take().unwrap(),
            };
            device.create_texture(&desc)
        };

        let view = {
            let label = self.label.map(|mut s| {
                let mut s = s.to_string();
                s.push_str(TEXTURE_VIEW_LABEL);
                s
            });
            let desc = TextureViewDescriptor {
                label: label.as_deref(),
                ..Default::default()
            };
            texture.create_view(&desc)
        };

        TextureBind { texture, view }
    }
}

// 从 bevy_render::texture::image 里拷贝过来的

/// Information about the pixel size in bytes and the number of different components.
pub struct PixelInfo {
    /// The size of a component of a pixel in bytes.
    pub type_size: usize,
    /// The amount of different components (color channels).
    pub num_components: usize,
}

/// Extends the wgpu [`TextureFormat`] with information about the pixel.
pub trait TextureFormatPixelInfo {
    /// Returns the pixel information of the format.
    fn pixel_info(&self) -> PixelInfo;
    /// Returns the size of a pixel of the format.
    fn pixel_size(&self) -> usize {
        let info = self.pixel_info();
        info.type_size * info.num_components
    }
}

impl TextureFormatPixelInfo for TextureFormat {
    #[allow(clippy::match_same_arms)]
    fn pixel_info(&self) -> PixelInfo {
        let type_size = match self {
            // 8bit
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb => 1,

            // 16bit
            TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Float
            | TextureFormat::R16Unorm
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Unorm
            | TextureFormat::Rg16Float
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float => 2,

            // 32bit
            TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float
            | TextureFormat::Depth32Float => 4,

            // special cases
            TextureFormat::Rgb9e5Ufloat => 4,
            TextureFormat::Rgb10a2Unorm => 4,
            TextureFormat::Rg11b10Float => 4,
            TextureFormat::Depth24Plus => 3, // FIXME is this correct?
            TextureFormat::Depth24PlusStencil8 => 4,
            // TODO: this is not good! this is a temporary step while porting bevy_render to direct wgpu usage
            _ => panic!("cannot get pixel info for type"),
        };

        let components = match self {
            TextureFormat::R8Unorm
            | TextureFormat::R8Snorm
            | TextureFormat::R8Uint
            | TextureFormat::R8Sint
            | TextureFormat::R16Uint
            | TextureFormat::R16Sint
            | TextureFormat::R16Unorm
            | TextureFormat::R16Float
            | TextureFormat::R32Uint
            | TextureFormat::R32Sint
            | TextureFormat::R32Float => 1,

            TextureFormat::Rg8Unorm
            | TextureFormat::Rg8Snorm
            | TextureFormat::Rg8Uint
            | TextureFormat::Rg8Sint
            | TextureFormat::Rg16Uint
            | TextureFormat::Rg16Sint
            | TextureFormat::Rg16Unorm
            | TextureFormat::Rg16Float
            | TextureFormat::Rg32Uint
            | TextureFormat::Rg32Sint
            | TextureFormat::Rg32Float => 2,

            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float => 4,

            // special cases
            TextureFormat::Rgb9e5Ufloat
            | TextureFormat::Rgb10a2Unorm
            | TextureFormat::Rg11b10Float
            | TextureFormat::Depth32Float
            | TextureFormat::Depth24Plus
            | TextureFormat::Depth24PlusStencil8 => 1,
            // TODO: this is not good! this is a temporary step while porting bevy_render to direct wgpu usage
            _ => panic!("cannot get pixel info for type"),
        };

        PixelInfo {
            type_size,
            num_components: components,
        }
    }
}
