struct CameraUniformData {
    /// Size of the viewport.
    viewport_size: vec2f,
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f
}

@group(0) @binding(0)
var<uniform> camera: CameraUniformData;

/// Contains the static transformations, i.e. the transformations applied
/// to the character when no animations are playing. These are required to
/// make the character render properly, as the vertex positions are in bone space.
@group(1) @binding(0)
var<storage, read> static_bind_poses: array<mat4x4f>;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    /// Indices of the bones that this vertex is connected to.
    @location(2) bone_indices: vec4u,
    /// Weights for the indices above.
    @location(3) bone_weights: vec4f
}

struct VertexOutput {
    @builtin(position) vertex: vec4f,
    @location(0) normal: vec3f,
    @location(1) @interpolate(flat) bone_index: u32
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var pos = vec4f(0.0);
    var nrm = vec3f(0.0);

    for (var i = 0u; i < 1u; i += 1) {
        let weight = input.bone_weights[i];
        if (weight > 0.0) {
            let transform = static_bind_poses[input.bone_indices[i]];
            pos += weight * (transform * vec4f(input.position, 1.0));
            nrm += weight * (mat3x3f(transform[0].xyz, transform[1].xyz, transform[2].xyz) * input.normal);
        }
    }

    var output: VertexOutput;
    output.vertex = camera.view_proj * pos;
    output.normal = normalize(nrm);
    output.bone_index = input.bone_indices[0];
    return output;
}

fn linear_to_srgb(color: vec3f) -> vec3f {
    return 1.055 * pow(color, vec3f(1.0 / 2.4)) - 0.055;
}

fn compute_diffuse(normal: vec3f) -> vec3f {
    let sunDirection = vec3f(0.0, -1.0, 1.0);
    let dot = dot(normal, sunDirection);

    return vec3f(normal * 0.5 + 0.5) * dot;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4f {
    var color: vec3f = input.normal * 0.5 + 0.5;
    if (input.bone_index == 1 || input.bone_index == 2 || input.bone_index == 3) {
        // left leg (red)
        color = vec3f(1.0, 0.0, 0.0);
    } else if (input.bone_index == 4 || input.bone_index == 5 || input.bone_index == 6) {
        // right leg (green)
        color = vec3f(0.0, 1.0, 0.0);
    } else if (input.bone_index == 8 || input.bone_index == 9 || input.bone_index == 10) {
        // left arm (blue)
        color = vec3f(0.0, 0.0, 1.0);
    }

    // Convert the linear colours to SRGB.
    // Without this, the colours will look very washed out in the editor.
    let srgb = linear_to_srgb(color);
    return vec4f(srgb, 1.0);
}
