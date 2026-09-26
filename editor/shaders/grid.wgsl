struct CameraUniform {
    viewport_size: vec4f,
    view_proj: mat4x4f,
    inv_view_proj: mat4x4f
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

const GRID_SPACING: f32 = 1.0;
const GRID_COLOR: vec4f = vec4f(0.5);

struct VertexOutput {
    @builtin(position) position: vec4f,
}

/// Generates 2D coordinates [-1.0, 1.0] from index 0, 1, 2
fn unproject_point(x: f32, y: f32, z: f32, inv_vp: mat4x4f) -> vec3f {
    let unprojected = inv_vp * vec4f(x, y, z, 1.0);
    return unprojected.xyz / unprojected.w;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Convert vertex index to NDC points such that the triangle covers the entire screen.
    // Index 0 converts to vec2(-1.0, -1.0).
    // Index 1 converts to vec2(3.0, -1.0)
    // Index 2 converts to vec2(-1.0, 3.0)
    let x = f32(vertex_index & 1) * 4.0 - 1.0;
    let y = f32(vertex_index >> 1) * 4.0 - 1.0;

    out.position = vec4f(x, y, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let ndc = vec2f(
        (in.position.x / camera.viewport_size.x) * 2.0 - 1.0,
        1.0 - (in.position.y / camera.viewport_size.y) * 2.0
    );

    let near = camera.inv_view_proj * vec4f(ndc, 0.0, 1.0);
    let far = camera.inv_view_proj * vec4f(ndc, 1.0, 1.0);

    let near_world = near.xyz / near.w;
    let far_world = far.xyz / far.w;

    let ray_orig = near_world;
    let ray_dir = normalize(far_world - near_world);

    // Intersect with y = 0
    let t = -ray_orig.y / ray_dir.y;

    if t < 0.0 {
        discard;
    }

    let world_pos = ray_orig + ray_dir * t;

    let grid_uv = world_pos.xz / GRID_SPACING;
    let grid_line = abs(fract(grid_uv - 0.5) * 0.5);

    let der = fwidth(world_pos.xz);
    let line_thickness = grid_line / der;

    let min_dist = min(line_thickness.x, line_thickness.y);

    // Step alpha such that only the grid lines are opaque.
    let line_alpha = 1.0 - smoothstep(0.0, 1.0, min_dist);

    // Then add another smoothstep to fade out the grid lines in the
    // distance. This reduces the noise in the distance.
    let dist_alpha = 1.0 - smoothstep(0.0, 50.0, t);

    return vec4f(GRID_COLOR.rgb, GRID_COLOR.a * line_alpha * dist_alpha);
}
