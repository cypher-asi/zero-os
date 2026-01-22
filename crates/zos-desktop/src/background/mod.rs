//! Desktop Background Renderer
//!
//! WebGPU-based fullscreen backgrounds with multiple procedural shaders.
//! Backgrounds can be switched at runtime without recompiling.
//!
//! ## Available Backgrounds
//!
//! - **Grain**: Subtle film grain on near-black (default)
//! - **Mist**: Two-pass animated smoke with glass overlay effect
//!   - Pass 1: Multi-layer smoke with parallax depth and volume lighting
//!   - Pass 2: Glass refraction, fresnel, specular highlights, dust/grain
//!
//! ## Design
//!
//! - Full-screen triangle rendered via vertex shader (no geometry needed)
//! - All procedural - no textures required
//! - Shared uniform buffer and bind group across all backgrounds
//! - Hot-swappable pipelines for instant background changes

mod init;
mod render;
mod renderer;
mod shaders;
mod types;
mod uniforms;

pub use renderer::BackgroundRenderer;
pub use types::BackgroundType;
pub use uniforms::Uniforms;
