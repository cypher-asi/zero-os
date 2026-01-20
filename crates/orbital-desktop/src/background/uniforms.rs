/// Uniform data sent to shaders
/// NOTE: This struct must match WGSL alignment requirements!
/// Total struct size must be 80 bytes (padded to 16-byte boundary).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub time: f32,                       // offset 0
    pub zoom: f32,                       // offset 4
    pub resolution: [f32; 2],            // offset 8
    pub viewport_center: [f32; 2],       // offset 16
    pub workspace_count: f32,            // offset 24
    pub active_workspace: f32,           // offset 28
    pub workspace_backgrounds: [f32; 4], // offset 32
    pub transitioning: f32,              // offset 48
    pub workspace_width: f32,            // offset 52
    pub workspace_height: f32,           // offset 56
    pub workspace_gap: f32,              // offset 60
    pub _pad: [f32; 4],                  // offset 64 - padding to 80 bytes
}

impl Uniforms {
    /// Create default uniforms
    pub fn default_with_resolution(width: u32, height: u32) -> Self {
        Self {
            time: 0.0,
            zoom: 1.0,
            resolution: [width as f32, height as f32],
            viewport_center: [0.0, 0.0],
            workspace_count: 2.0,
            active_workspace: 0.0,
            workspace_backgrounds: [0.0, 0.0, 0.0, 0.0],
            transitioning: 0.0,
            workspace_width: 1920.0,
            workspace_height: 1080.0,
            workspace_gap: 100.0,
            _pad: [0.0, 0.0, 0.0, 0.0],
        }
    }
}
