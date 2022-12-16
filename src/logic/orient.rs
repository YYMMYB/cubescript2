use nalgebra::*;

pub type Data = i32;
pub type CompressedData = u8;
pub type Code = u8;

// 编码格式: ("口" 是用来对齐的, 没意义)
// 二进制值    0       0       0       0       0       0
// 口口口口    ↑               ↑       ↑       ↑
// 字段名口    |axis           |flip   |sign   |rot
// 取值范围    |0..3           |0,1    |0,1    |0..4
#[derive(Debug)]
pub struct Orient<T> {
    pub rot: T,  // 沿着法线旋转几个90度, 取值是: 0,1,2,3
    pub sign: T, // 正轴还是负轴的方向, 取值是: 0(不变),1(变), 主要为了000000代表单位变换
    pub flip: T, // 是否进行平面翻转(沿着(法线±1)%3的轴翻转, 具体沿着哪个轴还没定), 取值是: 0,1
    pub axis: T, // xyz哪个轴, 取值是: 0,1,2
}

impl Orient<Data> {
    pub fn compressed(&self) -> Orient<CompressedData> {
        Orient::<CompressedData> {
            rot: self.rot as CompressedData,
            axis: self.axis as CompressedData,
            sign: self.sign as CompressedData,
            flip: self.flip as CompressedData,
        }
    }

    // 注意是 3x3 的矩阵, 而非 4x4 的. 因为没有位移
    pub fn to_matrix(&self) -> Matrix3<f32> {
        // axis0 代表 norm 的方向, 实现上是把原 正x轴 映射到 axis0 * sign
        // axis1 代表 up 的方向, 实现上是把原 正y轴 映射到 axis1 * sign
        // axis2 代表 剩下的那个方向, 实现上是把原 正z轴 映射到 axis2 * sign
        // signN 代表对应轴的方向, 1(不变, 即:正到正), -1(变, 即:正到负),
        // 这是为了在简单的编码解码上,可以让000000代表单位变换.

        let axis0 = axis_add(self.axis, 0);
        let sign0 = (self.sign * 2 - 1) * -1;
        // 列出来就完了, 整什么算法
        let rot = self.rot as usize;
        const P1_AXIS_LOOP: [i32; 4] = [-1, -2, -1, -2];
        const N1_AXIS_LOOP: [i32; 4] = [-1, -2, -1, -2];
        const P2_AXIS_LOOP: [i32; 4] = [1, 2, 1, 2]; // * -1
        const N2_AXIS_LOOP: [i32; 4] = [1, 2, 1, 2]; // * -1
        const P1_SIGN_LOOP: [i32; 4] = [1, 1, -1, -1];
        const N1_SIGN_LOOP: [i32; 4] = [1, -1, -1, 1]; // 逆序 或 idx+1
        const P2_SIGN_LOOP: [i32; 4] = [1, -1, -1, 1]; // 逆序 或 idx+1
        const N2_SIGN_LOOP: [i32; 4] = [-1, -1, 1, 1]; // * -1
        let a1l = if sign0 > 0 {
            P1_AXIS_LOOP
        } else {
            N1_AXIS_LOOP
        };
        let s1l = if sign0 > 0 {
            P1_SIGN_LOOP
        } else {
            N1_SIGN_LOOP
        };
        let axis1 = axis_add(axis0, a1l[rot]);
        let sign1 = sign0 * s1l[rot];
        let a2l = if sign0 > 0 {
            P2_AXIS_LOOP
        } else {
            N2_AXIS_LOOP
        };
        let s2l = if sign0 > 0 {
            P2_SIGN_LOOP
        } else {
            N2_SIGN_LOOP
        };
        let axis2 = axis_add(axis0, a2l[rot]);
        let sign2 = sign0 * s2l[rot];
        let mut m: [[f32; 3]; 3] = Default::default();
        // 第3列
        m[2][axis2 as usize] = sign2 as f32;
        // 第2列
        m[1][axis1 as usize] = sign1 as f32;
        // 第1列
        m[0][axis0 as usize] = sign0 as f32;

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

impl Orient<CompressedData> {
    const ROT_BITS: u8 = 2;
    const FLIP_BITS: u8 = 1;
    const SIGN_BITS: u8 = 1;

    pub fn uncompress(&self) -> Orient<Data> {
        Orient::<Data> {
            rot: self.rot as Data,
            axis: self.axis as Data,
            sign: self.sign as Data,
            flip: self.flip as Data,
        }
    }

    pub fn encode(&self) -> Code {
        let mut ret = self.rot;
        ret |= self.sign << Self::ROT_BITS;
        ret |= self.flip << (Self::SIGN_BITS + Self::ROT_BITS);
        ret |= self.axis << (Self::FLIP_BITS + Self::SIGN_BITS + Self::ROT_BITS);
        ret
    }
    pub fn decode(code: Code) -> Self {
        let rot = code & ((1 << Self::ROT_BITS) - 1);
        let sign = (code >> Self::ROT_BITS) & ((1 << Self::SIGN_BITS) - 1);
        let flip = (code >> (Self::SIGN_BITS + Self::ROT_BITS)) & ((1 << Self::FLIP_BITS) - 1);
        let axis = (code >> (Self::FLIP_BITS + Self::SIGN_BITS + Self::ROT_BITS));

        Self {
            rot,
            sign,
            axis,
            flip,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;
    const MAIN_DIR: [u8; 6] = [0, 4, 16, 20, 32, 36];
    fn show<T: Display>(c: u8, o: &Orient<T>) {
        println!(
            "code:{:06b};\tsign:{:}\taxis:{:}\trot:{:}\tflip:{:}",
            c, o.sign, o.axis, o.rot, o.flip
        )
    }
    fn show_decode() {
        let codes = MAIN_DIR;
        for c in codes {
            let o = Orient::<CompressedData>::decode(c);
            show(c, &o);
        }
    }

    fn show_to_mat() {
        let codes = MAIN_DIR;
        for c in codes {
            let o = Orient::<CompressedData>::decode(c).uncompress();
            show(c, &o);
            let m = o.to_matrix().to_homogeneous();
            println!("{}", m);
        }
    }
}
