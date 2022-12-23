use crate::*;
use image::*;

pub fn generate_rgba8_mip_map(image: Image, mip_map_count: u32) -> Result<ImageMipMap> {
    match image.format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint => {}
        _ => panic!("这是rgba8专用的生成mipmap的方法, 否则得做映射很麻烦, 先这样"),
    };
    let format = image.format;
    let size = Extent3d {
        width: image.width,
        height: image.height,
        depth_or_array_layers: 1,
    };
    let mut mips = vec![ImageBuffer::from_vec(image.width, image.height, image.data).unwrap()];
    for mip_i in 1..mip_map_count {
        let mip_size = size.mip_level_size(mip_i, false);
        let width = mip_size.width;
        let height = mip_size.height;
        let mut mip = ImageBuffer::new(width, height);
        for x in 0..width {
            for y in 0..height {
                let mut sum = Rgba([0u8; 4]);
                for dx in 0..2 {
                    for dy in 0..2 {
                        let p: &Rgba<u8> = mips[mip_i as usize - 1].get_pixel(x * 2 + dx, y * 2 + dy);
                        for c in 0..4 {
                            sum.0[c] += p[c] / 4;
                        }
                    }
                }
                mip.put_pixel(x, y, sum);
            }
        }
        mips.push(mip);
    }
    let mips: Vec<_> = mips.into_iter().map(|mip| mip.into_raw()).collect();
    Ok(ImageMipMap {
        data: mips,
        width: image.width,
        height: image.height,
        format,
    })
}
