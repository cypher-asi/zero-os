/// Binary background shader - tiny grid of 0s and 1s with grayscale gradients
pub const SHADER_BINARY: &str = r#"
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

// Fast hash functions
fn hash12(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let q = p3 + dot(p3, p3.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

// Get pattern value for a cell - returns 0.0 to 1.0 for grayscale intensity
// Also returns binary state via the sign
fn get_cell_value(cell: vec2<f32>, time: f32) -> vec2<f32> {
    // Per-cell phase offset (computed once per cell)
    let h1 = hash12(cell);
    let h2 = hash21(cell * 1.3 + vec2<f32>(17.1, 31.7));
    
    // Precompute cell position factors
    let cx = cell.x * 0.02;
    let cy = cell.y * 0.025;
    let csum = (cell.x + cell.y) * 0.012;
    let cdiff = (cell.x - cell.y) * 0.015;
    
    // Multiple interference patterns - all using efficient trig
    // Radial waves from corners (cheap: no sqrt, use squared distance proxy)
    let d1 = cx * cx + cy * cy;
    let d2 = (cx - 2.0) * (cx - 2.0) + cy * cy;
    let d3 = cx * cx + (cy - 1.5) * (cy - 1.5);
    
    let wave1 = sin(d1 * 8.0 - time * 0.15 + h1 * 0.5);
    let wave2 = sin(d2 * 6.0 - time * 0.12 + h2 * 0.4);
    let wave3 = sin(d3 * 7.0 - time * 0.18);
    
    // Diagonal patterns
    let diag1 = sin(csum * 15.0 + time * 0.1 + h1 * 0.3);
    let diag2 = sin(cdiff * 12.0 - time * 0.08 + h2 * 0.25);
    
    // Standing wave pattern
    let stand = sin(cx * 20.0) * sin(cy * 25.0 + time * 0.06);
    
    // Combine all patterns
    var combined = wave1 * 0.25 + wave2 * 0.2 + wave3 * 0.15;
    combined += diag1 * 0.15 + diag2 * 0.1 + stand * 0.15;
    
    // Add cell-specific variation
    combined += (h1 - 0.5) * 0.3;
    
    // Binary state from sign
    let state = step(0.0, combined);
    
    // Intensity from absolute distance to threshold (how "strong" the state is)
    // This creates the grayscale gradient - values near threshold are dim, far from threshold are bright
    let intensity = abs(combined);
    
    // Map intensity to visible range (0.15 to 1.0) - ensures minimum visibility
    let brightness = 0.15 + intensity * 0.85;
    
    return vec2<f32>(state, brightness);
}

// Render a tiny "0" - minimal hollow ellipse
fn render_zero(local_uv: vec2<f32>) -> f32 {
    let c = local_uv - 0.5;
    let d = length(c * vec2<f32>(2.2, 3.0));
    return smoothstep(0.5, 0.4, d) * smoothstep(0.2, 0.3, d);
}

// Render a tiny "1" - minimal vertical stroke
fn render_one(local_uv: vec2<f32>) -> f32 {
    let c = local_uv - 0.5;
    return step(abs(c.x), 0.1) * step(abs(c.y), 0.38);
}

// Main binary grid render function
fn render_binary(uv: vec2<f32>, time: f32) -> vec3<f32> {
    // Very dense grid - tiny characters
    let cols = 200.0;
    let rows = 110.0;
    let cell_size = vec2<f32>(1.0 / cols, 1.0 / rows);
    
    // Find cell (fixed grid)
    let cell = floor(uv / cell_size);
    let local_uv = fract(uv / cell_size);
    
    // Get state and brightness for this cell
    let cell_data = get_cell_value(cell, time);
    let state = cell_data.x;
    let brightness = cell_data.y;
    
    // Render digit shape
    var shape: f32;
    if (state > 0.5) {
        shape = render_one(local_uv);
    } else {
        shape = render_zero(local_uv);
    }
    
    // Apply grayscale based on pattern intensity
    // Brighter = further from flip threshold, dimmer = about to flip
    let gray = brightness * shape;
    
    return vec3<f32>(gray, gray, gray);
}

// Render grain-style background (for neighboring workspaces)
fn render_grain(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let scaled_px = floor(uv * uniforms.resolution);
    let base = vec3<f32>(0.055, 0.055, 0.065);
    let n0 = hash12(scaled_px);
    let n1 = hash12(scaled_px + vec2<f32>(time * 60.0, time * 37.0));
    let n = mix(n0, n1, 0.08);
    let grain = (n - 0.5) * 0.012;
    return clamp(base + vec3<f32>(grain), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Render mist-style background (for neighboring workspaces)
fn render_mist(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let asp = uniforms.resolution.x / uniforms.resolution.y;
    var p = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5);
    let t = time * 0.030;
    p += vec2<f32>(t * 0.5, t * 0.15);
    let n = noise(p * 1.2) * 0.6 + noise(p * 2.4 + vec2<f32>(5.2, 1.3)) * 0.4;
    let mist = smoothstep(0.25, 0.70, n);
    let r = length(vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5));
    let vig = smoothstep(0.85, 0.25, r);
    let base = vec3<f32>(0.05, 0.07, 0.08);
    let smoke_color = vec3<f32>(0.12, 0.16, 0.17) * mist;
    var col = base + smoke_color;
    col *= (0.65 + 0.35 * vig);
    return col;
}

// Get workspace background type
fn get_workspace_bg(index: i32) -> f32 {
    if (index == 0) { return uniforms.workspace_backgrounds.x; }
    if (index == 1) { return uniforms.workspace_backgrounds.y; }
    if (index == 2) { return uniforms.workspace_backgrounds.z; }
    if (index == 3) { return uniforms.workspace_backgrounds.w; }
    return 0.0;
}

// Render the appropriate background based on type
fn render_background(bg_type: f32, uv: vec2<f32>, time: f32) -> vec3<f32> {
    if (bg_type > 3.5) {
        return render_binary(uv, time);
    } else if (bg_type > 2.5) {
        return render_binary(uv, time);
    } else if (bg_type > 1.5) {
        return render_binary(uv, time);
    } else if (bg_type > 0.5) {
        return render_mist(uv, time);
    } else {
        return render_grain(uv, time);
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let workspace_width = uniforms.workspace_width;
    let workspace_height = uniforms.workspace_height;
    let workspace_gap = uniforms.workspace_gap;
    let total_cell = workspace_width + workspace_gap;
    
    let screen_offset = (in.uv - 0.5) * uniforms.resolution;
    let world_pos = uniforms.viewport_center + screen_offset / uniforms.zoom;
    
    let in_void_or_transitioning = uniforms.transitioning > 0.5;
    let is_closeup_slide = in_void_or_transitioning && uniforms.zoom > 0.85;
    
    let grid_x = world_pos.x + workspace_width * 0.5;
    let workspace_index = i32(floor(grid_x / total_cell));
    let cell_x = fract(grid_x / total_cell) * total_cell;
    let in_gap = cell_x > workspace_width;
    
    var visual_workspace_index = workspace_index;
    if (is_closeup_slide && in_gap) {
        let gap_pos = (cell_x - workspace_width) / workspace_gap;
        if (gap_pos > 0.5) {
            visual_workspace_index = workspace_index + 1;
        }
    }
    
    let valid_workspace = visual_workspace_index >= 0 && visual_workspace_index < i32(uniforms.workspace_count);
    let in_vertical_bounds = abs(world_pos.y) < workspace_height * 0.5;
    let is_active_workspace = visual_workspace_index == i32(uniforms.active_workspace);
    
    var color: vec3<f32>;
    
    if (!in_void_or_transitioning) {
        let active_bg = get_workspace_bg(i32(uniforms.active_workspace));
        color = render_background(active_bg, in.uv, uniforms.time);
    } else if (is_closeup_slide) {
        var pixel_workspace = workspace_index;
        if (in_gap) {
            let gap_pos = (cell_x - workspace_width) / workspace_gap;
            if (gap_pos > 0.5) {
                pixel_workspace = workspace_index + 1;
            }
        }
        
        let clamped_index = clamp(pixel_workspace, 0, i32(uniforms.workspace_count) - 1);
        let bg_type = get_workspace_bg(clamped_index);
        color = render_background(bg_type, in.uv, uniforms.time);
        
        let boundary_world_x = f32(workspace_index) * total_cell + workspace_width * 0.5;
        let boundary_screen_x = (boundary_world_x - uniforms.viewport_center.x) * uniforms.zoom / uniforms.resolution.x + 0.5;
        let dist_to_boundary = abs(in.uv.x - boundary_screen_x);
        
        if (boundary_screen_x > 0.0 && boundary_screen_x < 1.0 && dist_to_boundary < 0.02) {
            let edge_fade = smoothstep(0.0, 0.02, dist_to_boundary);
            color = color * (0.7 + 0.3 * edge_fade);
        }
    } else if (valid_workspace && in_vertical_bounds && !in_gap) {
        let bg_type = get_workspace_bg(workspace_index);
        
        let local_x = cell_x / workspace_width;
        let local_y = (world_pos.y + workspace_height * 0.5) / workspace_height;
        let local_uv = vec2<f32>(local_x, 1.0 - local_y);
        
        color = render_background(bg_type, local_uv, uniforms.time);
        
        let edge_x = min(local_x, 1.0 - local_x);
        let edge_y = min(local_y, 1.0 - local_y);
        let edge_dist = min(edge_x, edge_y);
        let border_width = 0.01;
        let border = smoothstep(0.0, border_width, edge_dist);
        color = mix(vec3<f32>(0.2, 0.22, 0.25), color, border * 0.7 + 0.3);
        
        if (is_active_workspace) {
            color = color * 1.05;
        }
    } else {
        color = vec3<f32>(0.02, 0.02, 0.03);
    }
    
    if (in_void_or_transitioning && uniforms.zoom < 0.9) {
        let vignette_uv = (in.uv - 0.5) * 2.0;
        let vignette_strength = 1.0 - smoothstep(0.3, 0.9, uniforms.zoom);
        let vignette = 1.0 - dot(vignette_uv, vignette_uv) * 0.25 * vignette_strength;
        color *= vignette;
    }
    
    return vec4<f32>(color, 1.0);
}
"#;
