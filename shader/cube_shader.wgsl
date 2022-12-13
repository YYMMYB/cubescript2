struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

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

@group(0) @binding(0)
var<uniform> view_mat: mat4x4<f32>; 
@group(0) @binding(1)
var<uniform> proj_mat: mat4x4<f32>; 

@vertex
fn vertex_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    var pv = proj_mat * view_mat;
    out.clip_position = pv * vec4<f32>(model.position, 1.0);
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    return out;
}

// Fragment shader

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = hsb2rgb(vec3<f32>(in.tex_coords.x,1.0,1.0));
    let color = pow(color,2.2 * vec3<f32>(1.0,1.0,1.0));

    let color = in.color;
    let color = vec4<f32>(color,1.0);
    
    return color;
}
