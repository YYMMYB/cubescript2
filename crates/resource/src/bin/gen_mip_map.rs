use resource::*;
use std::{fs::*, path::PathBuf, env};
use anyhow::*;

const SRC: &'static str = "../../image_raw/";
const DST: &'static str = "../../image/";

fn main() -> Result<()> {
    let cur_path = env::current_dir()?;
    let src = cur_path.clone().join(SRC);
    let dst = cur_path.clone().join(DST);
    dbg!(&src,&dst);
    let mip_map_count = 3;
    for e in read_dir(src.clone())?{
        let e = e?;
        let path = e.path();
        let m = metadata(path.clone())?;
        if m.is_file() {
            let img = Image::from_path(path.clone(), true)?;
            let mipmap = tools::generate_rgba8_mip_map(img, mip_map_count)?;
            let path = PathBuf::from(dst.clone()).join(path.file_stem().unwrap());
            mipmap.write_to_path(path);
        }
    }
    Ok(())
}