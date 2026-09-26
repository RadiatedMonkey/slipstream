struct CameraUniformData {
    viewport_size: vec4f,
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f
}

@group(0) @binding(0)
var<uniform> camera: CameraUniformData;

struct VertexInput {
    @location(0) position: vec3f
}

struct VertexOutput {
    @builtin(position) vertex: vec4f,
    @location(0) original: vec3f
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.vertex = camera.view_proj * vec4f(model.position, 1.0);
    out.original = model.position;
    return out;
}

fn linear_to_srgb(color: vec3f) -> vec3f {
    return 1.055 * pow(color, vec3f(1.0 / 2.4)) - 0.055;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    let color = input.original;

    // Convert the linear colours to SRGB.
    // Without this, the colours will look very washed out in the editor.
    let srgb = linear_to_srgb(color);
    return vec4f(srgb, 1.0);
}
