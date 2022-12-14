use super::*;

pub struct Scene {
    pub common_textures: Vec<Image>,
    pub cube: Mesh<cube::CubeVertx, cube::CubeInstance>
}

impl Scene {
    pub fn init() -> Self {
        todo!()
    }
}
