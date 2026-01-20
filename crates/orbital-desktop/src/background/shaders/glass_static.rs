/// Static glass overlay shader - rendered ONCE on init/resize
/// Outputs: RGB = additive color, A = UV distortion strength
pub const SHADER_GLASS_STATIC: &str = r#"
struct Uniforms {
    time: f32,
    zoom: f32,
    resolution: vec2<f32>,
    viewport_center: vec2<f32>,
    workspace_count: f32,
    active_workspace: f32,
    workspace_backgrounds: vec4<f32>,
    transitioning: f32,
    workspace_width: f32,
    workspace_height: f32,
    workspace_gap: f32,
    _pad: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let q = p3 + dot(p3, p3.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let asp = uniforms.resolution.x / uniforms.resolution.y;

    // Fresnel edge glow (static)
    let p = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5);
    let r = length(p);
    let fres = smoothstep(0.35, 0.95, r);
    var overlay = vec3<f32>(0.03, 0.04, 0.05) * fres;

    // Static specular highlight (diagonal streak)
    let highlight_pos = 0.3; // Fixed position
    let sweep = smoothstep(0.04, 0.0, abs((uv.x + uv.y * 0.2) - highlight_pos));
    overlay += vec3<f32>(0.06, 0.07, 0.08) * sweep * 0.3;

    // Dust/grain (static)
    let dust = (hash21(floor(uv * uniforms.resolution * 0.4)) - 0.5) * 0.008;
    overlay += vec3<f32>(dust);

    // Store UV distortion in alpha (for refraction effect)
    let px = uv * uniforms.resolution;
    let distort = (hash21(floor(px * 0.5)) - 0.5) * 0.004;

    return vec4<f32>(overlay, distort);
}
"#;
