/// Mist Pass 2: Composite smoke + static glass overlay
pub const SHADER_MIST_COMPOSITE: &str = r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(1) @binding(0) var smoke_tex: texture_2d<f32>;
@group(1) @binding(1) var smoke_samp: sampler;
@group(1) @binding(2) var glass_tex: texture_2d<f32>;
@group(1) @binding(3) var glass_samp: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vertex_index & 1u) * 4 - 1);
    let y = f32(i32(vertex_index >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Sample static glass overlay (RGB = color, A = distortion)
    let glass = textureSample(glass_tex, glass_samp, in.uv);
    
    // Apply static UV distortion from glass alpha
    let distorted_uv = in.uv + vec2<f32>(glass.a, glass.a * 0.7);
    
    // Sample smoke with distortion
    let smoke = textureSample(smoke_tex, smoke_samp, distorted_uv).rgb;
    
    // Composite: smoke + glass overlay (additive)
    let final_color = smoke + glass.rgb;
    
    return vec4<f32>(clamp(final_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;
