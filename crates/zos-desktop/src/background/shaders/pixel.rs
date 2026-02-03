/// Moving pixel background shader with angled grid pattern and color diversity
pub const SHADER_PIXEL: &str = r#"
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

fn hash12(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let q = p3 + dot(p3, p3.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let h1 = hash12(p);
    let h2 = hash21(p + vec2<f32>(43.0, 17.0));
    return vec2<f32>(h1, h2);
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

// Rotate a 2D point by angle (in radians)
fn rotate2d(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
}

// Get color from palette based on hash value
fn get_pixel_color(h: f32, brightness: f32) -> vec3<f32> {
    // Vibrant digital color palette
    let cyan = vec3<f32>(0.3, 0.9, 1.0);
    let blue = vec3<f32>(0.2, 0.5, 1.0);
    let white = vec3<f32>(0.95, 0.95, 1.0);
    let purple = vec3<f32>(0.6, 0.3, 0.9);
    let magenta = vec3<f32>(0.9, 0.2, 0.6);
    let green = vec3<f32>(0.2, 0.9, 0.5);
    
    var color: vec3<f32>;
    
    if (h < 0.2) {
        color = mix(cyan, blue, h * 5.0);
    } else if (h < 0.4) {
        color = mix(blue, white, (h - 0.2) * 5.0);
    } else if (h < 0.55) {
        color = mix(white, purple, (h - 0.4) * 6.67);
    } else if (h < 0.7) {
        color = mix(purple, cyan, (h - 0.55) * 6.67);
    } else if (h < 0.85) {
        // Occasional magenta accent
        color = mix(cyan, magenta, (h - 0.7) * 6.67);
    } else {
        // Rare green accent for digital feel
        color = mix(magenta, green, (h - 0.85) * 6.67);
    }
    
    return color * brightness;
}

// Render a single layer of angled pixels with dynamic on/off behavior
fn render_pixel_layer(
    uv: vec2<f32>,
    time: f32,
    grid_size: f32,
    angle: f32,
    speed: f32,
    intensity: f32,
    layer_offset: f32,
    density: f32
) -> vec3<f32> {
    let pixel_pos = uv * uniforms.resolution;
    
    // Apply slow movement offset before rotation
    let movement = vec2<f32>(time * speed * 15.0, time * speed * 8.0);
    let moved_pos = pixel_pos + movement;
    
    // Rotate the coordinate system for angled grid
    let rotated_pos = rotate2d(moved_pos, angle);
    
    // Create grid cells
    let cell = floor(rotated_pos / grid_size);
    let cell_pos = fract(rotated_pos / grid_size);
    
    // Hash for this cell
    let cell_hash = hash12(cell + vec2<f32>(layer_offset, layer_offset * 0.7));
    let color_hash = hash21(cell + vec2<f32>(42.0 + layer_offset, 17.0));
    let brightness_hash = hash12(cell + vec2<f32>(13.0, 29.0 + layer_offset));
    let blink_hash = hash12(cell + vec2<f32>(77.0 + layer_offset, 53.0));
    
    // Density control - skip some cells
    if (cell_hash > density) {
        return vec3<f32>(0.0);
    }
    
    // Pixel size varies slightly per cell
    let pixel_size = 0.15 + brightness_hash * 0.15;
    
    // Distance from cell center
    let center_dist = length(cell_pos - 0.5);
    
    // Sharp pixel with slight anti-aliasing
    let pixel = 1.0 - smoothstep(pixel_size - 0.05, pixel_size + 0.05, center_dist);
    
    // === DYNAMIC ON/OFF BEHAVIOR ===
    
    // Each pixel has its own blink frequency (very slow: 0.02 to 0.12 Hz = 8-50 second cycles)
    let blink_freq = 0.02 + blink_hash * 0.1;
    let blink_phase = cell_hash * 6.28318;
    
    // Create slow on/off pulses using sine wave
    let blink_wave = sin(time * blink_freq * 6.28318 + blink_phase);
    
    // Some pixels blink hard (full on/off), others shimmer softly
    let blink_intensity = blink_hash;  // 0-1: how "blinky" this pixel is
    
    // Hard blinkers: use threshold to create digital on/off
    let hard_blink = select(0.0, 1.0, blink_wave > (blink_hash * 0.8 - 0.4));
    
    // Soft shimmerers: smooth sine wave
    let soft_shimmer = 0.5 + 0.5 * blink_wave;
    
    // Mix between hard blink and soft shimmer based on pixel's character
    let blink_factor = mix(soft_shimmer, hard_blink, smoothstep(0.4, 0.7, blink_intensity));
    
    // Random flicker - very occasional flash (updates every 2 seconds)
    let flicker_seed = hash12(cell + vec2<f32>(floor(time * 0.5), layer_offset));
    let flicker = select(1.0, 1.4, flicker_seed > 0.98);  // 2% chance of subtle flash
    
    // Propagating wave that turns pixels on/off in sequence (very slow)
    let wave_center = vec2<f32>(
        sin(time * 0.02) * 50.0,
        cos(time * 0.015) * 40.0
    );
    let wave_dist = length(cell - wave_center);
    let wave_pulse = 0.8 + 0.2 * sin(wave_dist * 0.08 - time * 0.1);
    
    // Combine all effects
    let alive_factor = blink_factor * wave_pulse * flicker;
    
    // Base brightness with variation
    let base_brightness = 0.5 + brightness_hash * 0.5;
    let final_brightness = base_brightness * alive_factor;
    
    // Threshold to create more digital on/off feel
    let threshold_brightness = select(0.0, final_brightness, final_brightness > 0.25);
    
    // Get color from palette
    let color = get_pixel_color(color_hash, threshold_brightness);
    
    return color * pixel * intensity;
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

// Render dots-style background (for neighboring workspaces)
fn render_dots_simple(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let pixel_pos = uv * uniforms.resolution;
    let grid_size = 24.0;
    let cell = floor(pixel_pos / grid_size);
    let cell_center = (cell + 0.5) * grid_size;
    let dist = length(pixel_pos - cell_center);
    
    let cell_hash = hash12(cell);
    let shimmer = 0.5 + 0.5 * sin(time * 1.8 + cell_hash * 6.28318);
    let shimmer_intensity = 0.6 + 0.4 * shimmer;
    
    let color_hash = hash21(cell + vec2<f32>(42.0, 17.0));
    let cool_tint = vec3<f32>(0.65, 0.78, 0.92);
    let warm_tint = vec3<f32>(0.92, 0.72, 0.78);
    let dot_color = mix(cool_tint, warm_tint, color_hash);
    
    let dot = 1.0 - smoothstep(1.0, 2.0, dist);
    let base = vec3<f32>(0.018, 0.020, 0.028);
    
    return base + dot_color * dot * 0.16 * shimmer_intensity;
}

// Render pixel-style background with multiple angled layers
fn render_pixel(uv: vec2<f32>, time: f32) -> vec3<f32> {
    // Near-black background with slight blue tint
    let base = vec3<f32>(0.008, 0.010, 0.018);
    
    // Primary layer - main diagonal pattern (about 35 degrees)
    let primary = render_pixel_layer(
        uv, time,
        10.0,           // grid_size - dense pixels
        0.61,           // angle (~35 degrees in radians)
        0.0003,         // speed - extremely slow drift
        0.28,           // intensity - boosted for more presence
        0.0,            // layer_offset
        0.65            // density
    );
    
    // Secondary layer - different angle for depth (about -25 degrees)
    let secondary = render_pixel_layer(
        uv, time,
        12.0,           // slightly larger grid
        -0.44,          // angle (~-25 degrees)
        0.00025,        // movement speed
        0.20,           // intensity - boosted
        50.0,           // different layer offset
        0.55            // density
    );
    
    // Tertiary layer for extra depth (about 60 degrees)
    let tertiary = render_pixel_layer(
        uv, time,
        16.0,           // larger grid for distant feel
        1.05,           // angle (~60 degrees)
        0.0002,         // slow
        0.12,           // subtle but visible
        100.0,          // different offset
        0.40            // moderate density
    );
    
    // Fourth layer - adds more complexity (about -70 degrees)
    let quaternary = render_pixel_layer(
        uv, time,
        8.0,            // smaller grid - more detailed
        -1.22,          // angle (~-70 degrees)
        0.00035,        // slightly faster
        0.15,           // moderate intensity
        150.0,          // different offset
        0.45            // density
    );
    
    // Composite all layers
    var color = base + primary + secondary + tertiary + quaternary;
    
    // Subtle vignette for depth
    let asp = uniforms.resolution.x / uniforms.resolution.y;
    let centered_uv = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5);
    let vignette_dist = length(centered_uv);
    let vignette = 1.0 - smoothstep(0.5, 1.2, vignette_dist) * 0.15;
    
    color *= vignette;
    
    return color;
}

// Get workspace background type (0=grain, 1=mist, 2=dots, 3=pixel)
fn get_workspace_bg(index: i32) -> f32 {
    if (index == 0) { return uniforms.workspace_backgrounds.x; }
    if (index == 1) { return uniforms.workspace_backgrounds.y; }
    if (index == 2) { return uniforms.workspace_backgrounds.z; }
    if (index == 3) { return uniforms.workspace_backgrounds.w; }
    return 0.0;
}

// Render the appropriate background based on type
// 0=grain, 1=mist, 2=dots, 3=pixel, 4=mosaic, 5=binary
fn render_background(bg_type: f32, uv: vec2<f32>, time: f32) -> vec3<f32> {
    if (bg_type > 4.5) {
        // Binary (5) - fallback to grain
        return render_grain(uv, time);
    } else if (bg_type > 3.5) {
        // Mosaic (4) - fallback to grain
        return render_grain(uv, time);
    } else if (bg_type > 2.5) {
        // Pixel (3)
        return render_pixel(uv, time);
    } else if (bg_type > 1.5) {
        // Dots (2)
        return render_dots_simple(uv, time);
    } else if (bg_type > 0.5) {
        // Mist (1)
        return render_mist(uv, time);
    } else {
        // Grain (0)
        return render_grain(uv, time);
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Workspace layout from uniforms (must match Rust desktop engine)
    let workspace_width = uniforms.workspace_width;
    let workspace_height = uniforms.workspace_height;
    let workspace_gap = uniforms.workspace_gap;
    let total_cell = workspace_width + workspace_gap;
    
    // Calculate world position based on viewport
    let screen_offset = (in.uv - 0.5) * uniforms.resolution;
    let world_pos = uniforms.viewport_center + screen_offset / uniforms.zoom;
    
    // transitioning > 0.5 means we're in void mode or transitioning between workspaces
    // When false (in workspace mode), always render full-screen regardless of zoom
    let in_void_or_transitioning = uniforms.transitioning > 0.5;
    
    // Close-up slide: zoom ~1.0 during transition = workspace-to-workspace slide
    // In this mode, we hide gaps/borders but keep the same position calculations
    let is_closeup_slide = in_void_or_transitioning && uniforms.zoom > 0.85;
    
    // Determine which workspace this pixel belongs to (always use real gap for math)
    let grid_x = world_pos.x + workspace_width * 0.5;
    let workspace_index = i32(floor(grid_x / total_cell));
    let cell_x = fract(grid_x / total_cell) * total_cell;
    
    // Check if we're in the gap between workspaces
    let in_gap = cell_x > workspace_width;
    
    // For close-up slides in the gap, figure out which workspace to extend
    var visual_workspace_index = workspace_index;
    if (is_closeup_slide && in_gap) {
        // In gap during slide - extend the workspace we're closer to
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
        // WORKSPACE MODE: Always render full-screen active workspace background
        // The background fills the entire viewport regardless of zoom level
        let active_bg = get_workspace_bg(i32(uniforms.active_workspace));
        color = render_background(active_bg, in.uv, uniforms.time);
    } else if (is_closeup_slide) {
        // CLOSE-UP SLIDE: Render workspaces sliding across screen
        // The boundary between workspaces moves across the screen as viewport pans
        // Each side of the boundary renders its workspace's background using screen UV
        // This creates a visible "wipe" transition effect
        
        // Determine which workspace this screen pixel shows based on world position
        var pixel_workspace = workspace_index;
        if (in_gap) {
            // In gap - extend the closer workspace
            let gap_pos = (cell_x - workspace_width) / workspace_gap;
            if (gap_pos > 0.5) {
                pixel_workspace = workspace_index + 1;
            }
        }
        
        let clamped_index = clamp(pixel_workspace, 0, i32(uniforms.workspace_count) - 1);
        let bg_type = get_workspace_bg(clamped_index);
        
        // Use screen UV so the pattern stays stable on each side
        // The sliding effect comes from the BOUNDARY moving across the screen
        color = render_background(bg_type, in.uv, uniforms.time);
        
        // Add a subtle vertical line at workspace boundaries to make the wipe visible
        // Calculate where the boundary appears in screen space
        let boundary_world_x = f32(workspace_index) * total_cell + workspace_width * 0.5;
        let boundary_screen_x = (boundary_world_x - uniforms.viewport_center.x) * uniforms.zoom / uniforms.resolution.x + 0.5;
        let dist_to_boundary = abs(in.uv.x - boundary_screen_x);
        
        // Draw a subtle edge effect near the boundary (only if boundary is on screen)
        if (boundary_screen_x > 0.0 && boundary_screen_x < 1.0 && dist_to_boundary < 0.02) {
            let edge_fade = smoothstep(0.0, 0.02, dist_to_boundary);
            color = color * (0.7 + 0.3 * edge_fade);
        }
    } else if (valid_workspace && in_vertical_bounds && !in_gap) {
        // VOID MODE: Show grid of all workspaces with gaps and borders
        let bg_type = get_workspace_bg(workspace_index);
        
        let local_x = cell_x / workspace_width;
        let local_y = (world_pos.y + workspace_height * 0.5) / workspace_height;
        let local_uv = vec2<f32>(local_x, 1.0 - local_y);
        
        color = render_background(bg_type, local_uv, uniforms.time);
        
        // Add border around workspaces
        let edge_x = min(local_x, 1.0 - local_x);
        let edge_y = min(local_y, 1.0 - local_y);
        let edge_dist = min(edge_x, edge_y);
        let border_width = 0.01;
        let border = smoothstep(0.0, border_width, edge_dist);
        color = mix(vec3<f32>(0.2, 0.22, 0.25), color, border * 0.7 + 0.3);
        
        // Highlight active workspace slightly
        if (is_active_workspace) {
            color = color * 1.05;
        }
    } else {
        // THE VOID - pure darkness (outside workspace bounds or in gap)
        color = vec3<f32>(0.02, 0.02, 0.03);
    }
    
    // Add subtle vignette when in void mode and zoomed out
    if (in_void_or_transitioning && uniforms.zoom < 0.9) {
        let vignette_uv = (in.uv - 0.5) * 2.0;
        let vignette_strength = 1.0 - smoothstep(0.3, 0.9, uniforms.zoom);
        let vignette = 1.0 - dot(vignette_uv, vignette_uv) * 0.25 * vignette_strength;
        color *= vignette;
    }
    
    return vec4<f32>(color, 1.0);
}
"#;
