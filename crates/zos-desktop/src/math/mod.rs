//! Core geometry types for the desktop environment
//!
//! These types provide basic 2D math operations for positioning,
//! sizing, and camera transformations.

mod vec2;
mod rect;
mod size;
mod camera;
mod style;

pub use vec2::Vec2;
pub use rect::Rect;
pub use size::Size;
pub use camera::Camera;
pub use style::{FrameStyle, FRAME_STYLE};
