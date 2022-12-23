use crate::render::{built_in::cube, RenderState};
use anyhow::*;
use nalgebra::Vector3;

pub struct Scene {
    pub cubes: cube::Mesh,
}
impl Scene {
    pub fn init(render: &mut RenderState) -> Result<Self> {
        let mut cubes = render.cube_pipeline.new_cube_mesh(&render.device)?;
        let r = 2;
        for x in -r..r {
            for y in -r..r {
                for z in -r..r {
                    let pos = Vector3::new(x as f32, y as f32, z as f32) * 3f32;
                    cubes.add_cube(pos);
                }
            }
        }
        Ok(Scene { cubes })
    }
}
