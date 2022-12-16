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
    tex_index: u32,
    rot_id: u32,
}

fn get_info(c: vec4<u32>) -> Info {
    var info: Info;
    info.exp = c.x;
    info.tex_index = c.y;
    info.rot_id = c.z;
    return info;
}

@group(1) @binding(0)
var<storage, read> rot_mat_array: array<mat4x4<f32>, 48>; 

// @group(1) @binding(0)
// var<uniform> ttt : f32; 

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
    var s = exp2(f32(info.exp));
    var pos = (s * model.position);

    var pos4 = vec4<f32>(pos, 1.0);
    var rot = rot_mat_array[info.rot_id];
    pos4 = (rot * pos4);
    pos = pos4.xyz;

    // pos = pos + vec3<f32>(0.0, ttt, 0.0);

    // instance 的位置
    var pos = pos + instance.position;
    // 相机 vp 矩阵
    var vp = proj_mat * view_mat;

    var out: VertexOutput;
    out.clip_position = vp * vec4<f32>(pos, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = instance.color;
    return out;
}

// Fragment shader

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = hsb2rgb(vec3<f32>(in.tex_coords.x, 1.0, 1.0));
    let color = pow(color, 2.2 * vec3<f32>(1.0, 1.0, 1.0));

    let color = in.color * color;
    let color = vec4<f32>(color, 1.0);

    return color;
}
