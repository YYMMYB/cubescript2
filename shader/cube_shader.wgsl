struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(2) info: vec4<u32>,
    @location(3) position: vec3<f32>,
    @location(4) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_idx: i32,
};

fn hsb2rgb(c: vec3<f32>) -> vec3<f32> {
    let a = ((c.x * 6.0 + vec3<f32>(0.0, 4.0, 2.0)) % 6.0) - 3.0;
    let b = abs(a) - 1.0 ;
    let rgb = clamp(b, vec3<f32>(0.0), vec3<f32>(1.0).xxx);
    //rgb = rgb*rgb*(3.0-2.0*rgb);
    return c.z * mix(vec3<f32>(1.0, 1.0, 1.0), rgb, c.y);
}

struct Info {
    exp: u32,
    rot_flip: u32,
    tex_index0: u32,
    tex_index1: u32,
}

fn get_info(c: vec4<u32>) -> Info {
    var info: Info;
    info.exp = c.x;
    info.rot_flip = c.y;
    info.tex_index0 = c.z;
    info.tex_index1 = c.w;
    return info;
}

@group(1) @binding(0)
var<uniform> rot_mat_array: array<mat4x4<f32>, 48>; 

@group(0) @binding(0)
var<uniform> view_mat: mat4x4<f32>; 
@group(0) @binding(1)
var<uniform> proj_mat: mat4x4<f32>; 

@vertex
fn vertex_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let info = get_info(instance.info);
    // instance 的旋转缩放
    let s = exp2(f32(info.exp));
    let pos = (s * model.position);

    let pos4 = vec4<f32>(pos, 1.0);
    let id = info.rot_flip >> 1u;
    let rot = rot_mat_array[id];
    let pos4 = (rot * pos4);
    let pos = pos4.xyz;

    // instance 的位置
    var pos = pos + instance.position;
    // 相机 vp 矩阵
    let vp = proj_mat * view_mat;

    var out: VertexOutput;
    out.clip_position = vp * vec4<f32>(pos, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = instance.color;
    out.tex_idx = 1;

    let flip = f32(info.rot_flip & 1u);
    out.tex_coords.x = flip + (1f - flip * 2f) * out.tex_coords.x;
    return out;
}

// Fragment shader

@group(1) @binding(1)
var tex_arr_samp : sampler;
@group(1) @binding(2)
var tex_arr: texture_2d_array<f32>;

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let color = hsb2rgb(vec3<f32>(in.tex_coords.x, 1.0, 1.0));
    let color = pow(color, 2.2 * vec3<f32>(1.0, 1.0, 1.0));

    let tx = textureSample(tex_arr, tex_arr_samp, in.tex_coords, in.tex_idx);

    let color = in.color * color;
    let color = vec4<f32>(color, 1.0) * tx ;

    return tx;
}
