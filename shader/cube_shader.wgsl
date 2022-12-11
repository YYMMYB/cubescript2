struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};


fn hsb2rgb(c: vec3<f32>) -> vec3<f32> {
    let a = ((c.x * 6.0 + vec3<f32>(0.0, 4.0, 2.0)) % 6.0) - 3.0;
    let b = abs(a) - 1.0 ;
    let rgb = clamp(b, vec3<f32>(0.0), vec3<f32>(1.0).xxx);
    //rgb = rgb*rgb*(3.0-2.0*rgb);
    return c.z * mix(vec3<f32>(1.0, 1.0, 1.0), rgb, c.y);
}


@group(1) @binding(0)
var<uniform> vp: mat4x4<f32>; 
@group(2) @binding(0)
var<uniform> test: mat4x4<f32>; 


@vertex
fn vertex_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {

    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.color = (vec4<f32>(1.0, 0.0, 0.0, 0.0) * test).xyz;
    out.clip_position = vp * vec4<f32>(model.position, 1.0);
    // out.clip_position = model_matrix * out.clip_position;
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse : sampler;


@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    // let color = vec4<f32>(in.color, 1.0);

    let color = hsb2rgb(vec3<f32>(in.tex_coords.x,1.0,1.0));
    let color = vec4<f32>(color,1.0);
    return color;
}
