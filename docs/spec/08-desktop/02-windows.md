# Window Management

**Component:** 10-desktop/02-windows  
**Status:** Specification

---

## Overview

The Window Manager handles window lifecycle, positioning, and focus.

---

## Window

```rust
/// Window identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Window state
pub struct Window {
    /// Unique identifier
    pub id: WindowId,
    
    /// Window title
    pub title: String,
    
    /// Position on canvas (top-left)
    pub position: Vec2,
    
    /// Size in logical pixels
    pub size: Size,
    
    /// Minimum size
    pub min_size: Size,
    
    /// Maximum size (None = unlimited)
    pub max_size: Option<Size>,
    
    /// Window state
    pub state: WindowState,
    
    /// Associated process
    pub process: ProcessId,
    
    /// Z-order (higher = on top)
    pub z_order: u32,
}

/// Window state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}
```

---

## Window Manager

```rust
/// Window manager
pub struct WindowManager {
    /// All windows
    windows: HashMap<WindowId, Window>,
    
    /// Focus stack (most recent first)
    focus_stack: Vec<WindowId>,
    
    /// Next window ID
    next_id: u64,
    
    /// Next z-order
    next_z: u32,
}

impl WindowManager {
    /// Create a new window
    pub fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        
        let window = Window {
            id,
            title: config.title,
            position: config.position.unwrap_or(self.cascade_position()),
            size: config.size,
            min_size: config.min_size.unwrap_or(Size::new(100.0, 100.0)),
            max_size: config.max_size,
            state: WindowState::Normal,
            process: config.process,
            z_order: self.next_z,
        };
        self.next_z += 1;
        
        self.windows.insert(id, window);
        self.focus_window(id);
        
        id
    }
    
    /// Close a window
    pub fn close_window(&mut self, id: WindowId) {
        self.windows.remove(&id);
        self.focus_stack.retain(|&w| w != id);
    }
    
    /// Focus a window
    pub fn focus_window(&mut self, id: WindowId) {
        // Remove from current position
        self.focus_stack.retain(|&w| w != id);
        
        // Add to front
        self.focus_stack.insert(0, id);
        
        // Update z-order
        if let Some(window) = self.windows.get_mut(&id) {
            window.z_order = self.next_z;
            self.next_z += 1;
        }
    }
    
    /// Get focused window
    pub fn focused(&self) -> Option<WindowId> {
        self.focus_stack.first().copied()
    }
    
    /// Move window
    pub fn move_window(&mut self, id: WindowId, position: Vec2) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.position = position;
        }
    }
    
    /// Resize window
    pub fn resize_window(&mut self, id: WindowId, size: Size) {
        if let Some(window) = self.windows.get_mut(&id) {
            // Clamp to min/max
            let width = size.width.max(window.min_size.width);
            let height = size.height.max(window.min_size.height);
            
            let width = if let Some(max) = window.max_size {
                width.min(max.width)
            } else {
                width
            };
            let height = if let Some(max) = window.max_size {
                height.min(max.height)
            } else {
                height
            };
            
            window.size = Size::new(width, height);
        }
    }
    
    /// Set window state
    pub fn set_state(&mut self, id: WindowId, state: WindowState) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.state = state;
        }
    }
    
    /// Get windows sorted by z-order
    pub fn windows_by_z(&self) -> Vec<&Window> {
        let mut windows: Vec<_> = self.windows.values().collect();
        windows.sort_by_key(|w| w.z_order);
        windows
    }
    
    fn cascade_position(&self) -> Vec2 {
        let offset = (self.windows.len() % 10) as f32 * 30.0;
        Vec2::new(100.0 + offset, 100.0 + offset)
    }
}

/// Window creation config
pub struct WindowConfig {
    pub title: String,
    pub position: Option<Vec2>,
    pub size: Size,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub process: ProcessId,
}
```

---

## Hit Testing

```rust
impl WindowManager {
    /// Find window at screen position
    pub fn window_at(&self, pos: Vec2, viewport: &Viewport) -> Option<WindowId> {
        let canvas_pos = viewport.screen_to_canvas(pos);
        
        // Check windows in reverse z-order (top first)
        for window in self.windows_by_z().iter().rev() {
            let rect = Rect::new(window.position, window.size);
            if rect.contains(canvas_pos) {
                return Some(window.id);
            }
        }
        
        None
    }
    
    /// Find window region at position
    pub fn window_region_at(&self, pos: Vec2, viewport: &Viewport) -> Option<(WindowId, WindowRegion)> {
        let canvas_pos = viewport.screen_to_canvas(pos);
        
        for window in self.windows_by_z().iter().rev() {
            if let Some(region) = window.region_at(canvas_pos) {
                return Some((window.id, region));
            }
        }
        
        None
    }
}

/// Window region for interaction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRegion {
    TitleBar,
    Content,
    CloseButton,
    MinimizeButton,
    MaximizeButton,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
}
```

---

*[Back to Desktop](README.md) | [Previous: Engine](01-engine.md) | [Next: Input](03-input.md)*
