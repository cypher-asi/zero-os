//! Drag state for input operations

use crate::math::{Size, Vec2};
use crate::window::{WindowId, WindowRegion};

/// Current drag operation state
#[derive(Clone, Debug)]
pub enum DragState {
    /// Panning the canvas
    PanCanvas {
        /// Start screen position
        start: Vec2,
        /// Viewport center at start
        start_center: Vec2,
    },
    /// Moving a window
    MoveWindow {
        /// Window being moved
        window_id: WindowId,
        /// Offset from window origin to cursor
        offset: Vec2,
    },
    /// Resizing a window
    ResizeWindow {
        /// Window being resized
        window_id: WindowId,
        /// Which resize handle
        handle: WindowRegion,
        /// Window position at start
        start_pos: Vec2,
        /// Window size at start
        start_size: Size,
        /// Mouse position at start (canvas coords)
        start_mouse: Vec2,
    },
}

impl DragState {
    /// Check if this is a canvas pan operation
    #[inline]
    pub fn is_pan(&self) -> bool {
        matches!(self, DragState::PanCanvas { .. })
    }

    /// Check if this is a window move operation
    #[inline]
    pub fn is_move(&self) -> bool {
        matches!(self, DragState::MoveWindow { .. })
    }

    /// Check if this is a window resize operation
    #[inline]
    pub fn is_resize(&self) -> bool {
        matches!(self, DragState::ResizeWindow { .. })
    }

    /// Get the window ID if this is a window operation
    pub fn window_id(&self) -> Option<WindowId> {
        match self {
            DragState::MoveWindow { window_id, .. } => Some(*window_id),
            DragState::ResizeWindow { window_id, .. } => Some(*window_id),
            _ => None,
        }
    }
}
