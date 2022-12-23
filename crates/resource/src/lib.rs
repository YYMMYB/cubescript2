use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    mem,
    num::NonZeroU32,
    path::Path, env,
};

use anyhow::*;
use image::{ImageBuffer, Rgba};
use wgpu::*;

pub mod tools;

pub enum ShaderData {
    Wgsl(String),
    SpirV(Vec<u32>),
}

impl ShaderData {
    pub fn as_shader_source(&self) -> ShaderSource {
        match self {
            ShaderData::Wgsl(d) => ShaderSource::Wgsl(d.into()),
            _ => {
                panic!("{}", UNSUPPORTED)
            }
        }
    }
}

pub struct Shader {
    pub data: ShaderData,
    enter_point: String,
}

impl Shader {
    pub const VS_FUNC_NAME: &str = "vertex_main";
    pub const FS_FUNC_NAME: &str = "fragment_main";
    pub fn enter_point(&self) -> &str {
        &self.enter_point
    }
}

impl Shader {
    pub fn from_path(path:impl AsRef<Path>, ty: ShaderType, enter_point:String) -> Result<Self> {
        dbg!(path.as_ref());
        let mut f = File::open(path)?;
        let data = match ty {
            ShaderType::Wgsl => {
                let mut buf = String::new();
                f.read_to_string(&mut buf)?;
                ShaderData::Wgsl(buf)
            }
            ShaderType::SpirV => {
                panic!("{}", UNSUPPORTED)
            }
        };
        Ok(Self {
            data,
            enter_point,
        })
    }
}

pub enum ShaderType {
    Wgsl,
    SpirV,
}

#[derive(Debug)]
pub struct Image {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}
impl Image {
    pub fn from_path(path: impl AsRef<Path>, srgb: bool) -> Result<Self> {
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

#[derive(Debug)]
pub struct ImageMipMap {
    pub data: Vec<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}
impl ImageMipMap {
    pub fn extent_3d(&self, mip_level: u32) -> Extent3d {
        let o = Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        };
        o.mip_level_size(mip_level, false)
    }

    pub fn mip_map_count(&self) -> u32 {
        self.data.len() as u32
    }

    pub fn from_path(path: impl AsRef<Path>, is_srgb: bool) -> Result<ImageMipMap> {
        let mut data = Vec::new();
        let mut width = None;
        let mut height = None;
        for e in fs::read_dir(path)? {
            let e = e?;
            let p = e.path();
            if p.is_file() {
                let level = p
                    .file_stem()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap()
                    .parse::<usize>()?;
                let img = image::open(p.clone())?;
                if level == 0 {
                    width = Some(img.width());
                    height = Some(img.height());
                };
                let buf = img.into_rgba8().into_raw();
                if data.len() < level + 1 {
                    for i in data.len()..level + 1 {
                        data.push(None);
                    }
                }
                data[level] = Some(buf);
            }
        }
        let data = data.into_iter().map(|d| d.unwrap()).collect();
        Ok(ImageMipMap {
            data,
            width: width.unwrap(),
            height: height.unwrap(),
            format: if is_srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
        })
    }

    pub fn write_to_path(mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = if path.as_ref().exists() {
            if path.as_ref().is_dir() {
                path
            } else {
                panic!("{:?} 是文件, 先删掉", path.as_ref())
            }
        } else {
            fs::create_dir(path.as_ref())?;
            path
        };
        let data = mem::replace(&mut self.data, Vec::new());
        let mut data: Vec<_> = data.into_iter().map(|x| Some(x)).collect();
        for i in 0..self.mip_map_count() {
            let size = self.extent_3d(i);
            let mut mip_path = path.as_ref().clone().join(i.to_string());
            mip_path.set_extension("png");
            let d = data[i as usize].take().unwrap();
            let img = ImageBuffer::<Rgba<u8>, _>::from_vec(size.width, size.height, d).unwrap();
            img.save(mip_path)?;
        }
        Ok(())
    }

    pub fn write_into_texture(&self, queue: &Queue, texture: &Texture, layer: u32) {
        dbg!(&self.format,&self.width,&self.height);
        let mut mip_level = 0;
        let mut i = 0;
        for d in &self.data {
            dbg!(&i, &d.len());
            i = i+1;
            // 参考 device.create_texture_with_data
            let e = self.extent_3d(mip_level);
            let phy_size = e.physical_size(self.format);
            let info = self.format.describe();

            let columns = phy_size.width / info.block_dimensions.0 as u32;
            let rows = phy_size.height / info.block_dimensions.1 as u32;
            let row_bytes = info.block_size as u32 * columns;

            let data_layout = ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(row_bytes).unwrap()),
                rows_per_image: Some(NonZeroU32::new(rows).unwrap()),
            };

            queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: mip_level,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: TextureAspect::All,
                },
                &d[..],
                data_layout,
                Extent3d {
                    width: columns,
                    height: rows,
                    depth_or_array_layers: 1,
                },
            );
            mip_level += 1;
        }
    }
}

const UNSUPPORTED: &'static str = "目前不支持";
const RESOURCE_UNLOADED: &'static str = "资源未加载";
