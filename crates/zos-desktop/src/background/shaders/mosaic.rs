/// Mosaic background shader - colorful cubes assembling into a grid with retro digital vibes
pub const SHADER_MOSAIC: &str = r#"
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
    let n = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(n) * 43758.5453);
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

// Muted retro digital color palette - darker, more subtle
fn get_retro_color(index: f32) -> vec3<f32> {
    let i = i32(index * 8.0) % 8;
    
    // Muted cyan/teal
    if (i == 0) { return vec3<f32>(0.0, 0.35, 0.38); }
    // Muted magenta/pink
    if (i == 1) { return vec3<f32>(0.4, 0.12, 0.28); }
    // Muted yellow/gold
    if (i == 2) { return vec3<f32>(0.42, 0.38, 0.08); }
    // Muted green
    if (i == 3) { return vec3<f32>(0.1, 0.38, 0.14); }
    // Muted purple
    if (i == 4) { return vec3<f32>(0.25, 0.12, 0.4); }
    // Muted orange
    if (i == 5) { return vec3<f32>(0.42, 0.22, 0.06); }
    // Muted cool blue
    if (i == 6) { return vec3<f32>(0.14, 0.22, 0.4); }
    // Muted rose
    if (i == 7) { return vec3<f32>(0.4, 0.08, 0.22); }
    
    return vec3<f32>(0.3, 0.3, 0.3);
}

// Render a single cube/tile with subtle glow
fn render_cube(
    local_uv: vec2<f32>,
    cube_size: f32,
    color: vec3<f32>,
    shimmer: f32
) -> vec4<f32> {
    // Distance from center of cell (square/Chebyshev distance)
    let d = max(abs(local_uv.x), abs(local_uv.y));
    
    // Cube fill (square shape)
    let cube_edge = cube_size * 0.5;
    let fill = smoothstep(cube_edge + 0.02, cube_edge - 0.02, d);
    
    // Inner gradient for subtle depth
    let inner_grad = 0.7 + 0.3 * (1.0 - smoothstep(0.0, cube_edge, d));
    
    // Very subtle outer glow
    let glow = smoothstep(cube_edge + 0.06, cube_edge, d) * (1.0 - fill) * 0.15;
    
    // Combine - muted and subtle
    let cube_color = color * inner_grad * shimmer;
    let final_color = cube_color * fill + color * glow * 0.5;
    let alpha = fill + glow;
    
    return vec4<f32>(final_color, alpha);
}

// Main mosaic render function
fn render_mosaic(uv: vec2<f32>, time: f32) -> vec3<f32> {
    let asp = uniforms.resolution.x / uniforms.resolution.y;
    
    // Grid configuration - much smaller squares
    let grid_cols = 56.0;
    let grid_rows = grid_cols / asp;
    
    // Dark background
    var color = vec3<f32>(0.018, 0.02, 0.028);
    
    // Subtle radial gradient in background
    let center_uv = vec2<f32>((uv.x - 0.5) * asp, uv.y - 0.5);
    let center_dist = length(center_uv);
    color += vec3<f32>(0.01, 0.012, 0.018) * (1.0 - smoothstep(0.0, 0.9, center_dist));
    
    // Scale UV to aspect-corrected grid space
    let grid_uv = vec2<f32>(uv.x * grid_cols, uv.y * grid_cols / asp);
    
    // Current cell
    let cell = floor(grid_uv);
    
    // Smooth oscillating animation - tiles breathe in and out, never jump
    // Use sine wave for perfectly smooth back-and-forth motion
    let oscillation_period = 12.0;
    let base_phase = time * 6.28318 / oscillation_period;
    
    // Check neighboring cells too for smooth cube rendering
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let neighbor_cell = cell + vec2<f32>(f32(dx), f32(dy));
            
            // Skip cells outside reasonable bounds
            if (neighbor_cell.x < -1.0 || neighbor_cell.x > grid_cols + 1.0 ||
                neighbor_cell.y < -1.0 || neighbor_cell.y > grid_rows + 1.0) {
                continue;
            }
            
            // Per-cell random values (stable per cell)
            let cell_hash = hash12(neighbor_cell);
            let cell_hash2 = hash21(neighbor_cell + vec2<f32>(42.0, 17.0));
            let offset_hash = hash22(neighbor_cell + vec2<f32>(13.0, 29.0));
            
            // Phase offset based on position - creates wave pattern
            let cell_center = vec2<f32>(grid_cols * 0.5, grid_rows * 0.5);
            let dist_from_center = length(neighbor_cell - cell_center) / length(cell_center);
            let phase_offset = dist_from_center * 2.5 + cell_hash * 1.5;
            
            // Smooth sine oscillation - ranges from 0 to 1 and back
            // sin returns -1 to 1, we map to 0 to 1
            let oscillation = 0.5 + 0.5 * sin(base_phase + phase_offset);
            
            // Maximum scatter offset for this tile (consistent per tile)
            let scatter_amount = 0.35;
            let max_offset = (offset_hash - 0.5) * scatter_amount * 2.0;
            
            // Current offset oscillates between max_offset and zero
            let current_offset = max_offset * (1.0 - oscillation);
            
            // Calculate UV relative to this neighbor cell
            let rel_uv = grid_uv - neighbor_cell - 0.5 - current_offset;
            
            // Color from muted retro palette
            let color_idx = cell_hash;
            let cube_color = get_retro_color(color_idx);
            
            // Very subtle shimmer/pulse effect
            let shimmer_phase = cell_hash2 * 6.28318;
            let shimmer = 0.9 + 0.1 * sin(time * 1.2 + shimmer_phase);
            
            // Cube size - smaller with gaps
            let cube_size = 0.7 + cell_hash2 * 0.08;
            
            // Render the cube
            let cube = render_cube(rel_uv, cube_size, cube_color, shimmer);
            
            // Composite with alpha - more subtle blending
            color = mix(color, cube.rgb, cube.a * 0.85);
        }
    }
    
    // Very subtle scanlines for CRT feel
    let scanline = sin(uv.y * uniforms.resolution.y * 1.0) * 0.5 + 0.5;
    color *= 0.97 + scanline * 0.03;
    
    // Very subtle noise/grain
    let grain = (hash12(floor(uv * uniforms.resolution) + vec2<f32>(time * 10.0, 0.0)) - 0.5) * 0.01;
    color += vec3<f32>(grain);
    
    // Vignette - subtle darkening at edges
    let vig = 1.0 - smoothstep(0.4, 1.1, center_dist) * 0.35;
    color *= vig;
    
    return color;
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
        return render_mosaic(uv, time);
    } else if (bg_type > 2.5) {
        return render_mosaic(uv, time); // Use mosaic as fallback
    } else if (bg_type > 1.5) {
        return render_mosaic(uv, time);
    } else if (bg_type > 0.5) {
        return render_mist(uv, time);
    } else {
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
