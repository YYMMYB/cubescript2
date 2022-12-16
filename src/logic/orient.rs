use nalgebra::*;

pub type Data = i32;
pub type CompressedData = u8;
pub type Code = u8;
pub struct Orient<T> {
    pub rot: T,       // 沿着法线旋转几个90度, 取值是: 0,1,2,3
    pub norm_sign: T, // 正轴还是负轴的方向, 取值是: 0,1 (也可能改成-1,1)
    pub flip: T,      // 是否进行平面翻转(沿着(法线±1)%3的轴翻转, 具体沿着哪个轴还没定), 取值是: 0,1
    pub norm_axis: T, // xyz哪个轴, 取值是: 0,1,2
}

impl Orient<Data> {
    pub fn compressed(&self) -> Orient<CompressedData> {
        Orient::<CompressedData> {
            rot: self.rot as CompressedData,
            norm_axis: self.norm_axis as CompressedData,
            norm_sign: self.norm_sign as CompressedData,
            flip: self.flip as CompressedData,
        }
    }

    // 注意是 3x3 的矩阵, 而非 4x4 的. 因为没有位移
    pub fn to_matrix(&self) -> Matrix3<f32> {
        let zaxis = self.norm_axis;
        let zsign = self.norm_sign * 2 - 1;
        // 列出来就完了, 整什么算法
        let rot = self.rot as usize;
        const PY_AXIS_LOOP: [i32; 4] = [-1, -2, -1, -2];
        const NY_AXIS_LOOP: [i32; 4] = [-1, -2, -1, -2];
        const PX_AXIS_LOOP: [i32; 4] = [1, 2, 1, 2]; // * -1
        const NX_AXIS_LOOP: [i32; 4] = [1, 2, 1, 2]; // * -1
        const PY_SIGN_LOOP: [i32; 4] = [1, 1, -1, -1];
        const NY_SIGN_LOOP: [i32; 4] = [1, -1, -1, 1]; // 逆序 或 idx+1
        const PX_SIGN_LOOP: [i32; 4] = [1, -1, -1, 1]; // 逆序 或 idx+1
        const NX_SIGN_LOOP: [i32; 4] = [-1, -1, 1, 1]; // * -1
        let yal = if zsign > 0 {PY_AXIS_LOOP} else {NY_AXIS_LOOP};
        let ysl = if zsign > 0 {PY_SIGN_LOOP} else {NY_SIGN_LOOP};
        let yaxis = axis_add(zaxis, yal[rot]);
        let ysign = zsign * ysl[rot];
        let xal = if zsign > 0 {PX_AXIS_LOOP} else {NX_AXIS_LOOP};
        let xsl = if zsign > 0 {PX_SIGN_LOOP} else {NX_SIGN_LOOP};
        let xaxis = axis_add(zaxis, xal[rot]);
        let xsign = zsign * xsl[rot];
        let mut m: [[f32; 3]; 3] = Default::default();
        // 第3列
        m[2][zaxis as usize] = zsign as f32;
        // 第2列
        m[1][yaxis as usize] = ysign as f32;
        // 第1列
        m[0][xaxis as usize] = xsign as f32;
        
        Matrix3::from(m)
    }
}

fn axis_add(axis: i32, d: i32) -> i32 {
    let res = axis + d;
    if res < 0 {
        let abs = res.abs();
        3 - (abs % 3)
    } else {
        res % 3
    }
}

#[test]
fn test() {
    dbg!(axis_add(-12, 4));
}

impl Orient<CompressedData> {
    const ROT_BITS: u8 = 2;
    const FLIP_BITS: u8 = 1;
    const NORM_SIGN_BITS: u8 = 1;

    pub fn uncompress(&self) -> Orient<Data> {
        Orient::<Data> {
            rot: self.rot as Data,
            norm_axis: self.norm_axis as Data,
            norm_sign: self.norm_sign as Data,
            flip: self.flip as Data,
        }
    }
    pub fn encode(&self) -> Code {
        let mut ret = self.rot;
        ret |= self.norm_sign << Self::ROT_BITS;
        ret |= self.flip << (Self::NORM_SIGN_BITS + Self::ROT_BITS);
        ret |= self.norm_axis << (Self::FLIP_BITS + Self::NORM_SIGN_BITS + Self::ROT_BITS);
        ret
    }
    pub fn decode(code: Code) -> Self {
        let rot = code & ((1 << Self::ROT_BITS) - 1);
        let norm_sign = (code >> Self::ROT_BITS) & ((1 << Self::NORM_SIGN_BITS) - 1);
        let flip = (code >> (Self::NORM_SIGN_BITS + Self::ROT_BITS)) & ((1 << Self::FLIP_BITS) - 1);
        let norm_axis = (code >> (Self::FLIP_BITS + Self::NORM_SIGN_BITS + Self::ROT_BITS));

        Self {
            rot,
            norm_sign,
            norm_axis,
            flip,
        }
    }
}
