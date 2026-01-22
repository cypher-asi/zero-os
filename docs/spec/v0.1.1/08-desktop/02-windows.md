# Window Management

## Overview

Windows are the primary UI elements in Zero OS. Each window has a position, size, state, and associated application.

## Window Structure

```rust
pub struct Window {
    /// Unique window ID
    pub id: WindowId,
    
    /// Window title
    pub title: String,
    
    /// Position on canvas (world coordinates)
    pub position: Vec2,
    
    /// Current size
    pub size: Size,
    
    /// Minimum size constraint
    pub min_size: Option<Size>,
    
    /// Maximum size constraint
    pub max_size: Option<Size>,
    
    /// Window state (Normal, Minimized, Maximized)
    pub state: WindowState,
    
    /// Saved position/size for restore
    pub restore_bounds: Option<Rect>,
    
    /// Application ID
    pub app_id: String,
    
    /// Associated process ID (for lifecycle management)
    pub process_id: Option<u64>,
    
    /// Window type
    pub window_type: WindowType,
    
    /// Whether content is interactive (legacy)
    pub content_interactive: bool,
}

pub type WindowId = u64;

#[repr(u8)]
pub enum WindowState {
    Normal = 0,
    Minimized = 1,
    Maximized = 2,
}

pub enum WindowType {
    Standard,  // Full window with title bar
    Widget,    // Smaller, frameless or minimal frame
}
```

## WindowConfig

Configuration for creating windows:

```rust
pub struct WindowConfig {
    pub title: String,
    pub position: Option<Vec2>,      // None = auto-cascade
    pub size: Size,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub app_id: String,
    pub process_id: Option<u64>,
    pub content_interactive: bool,   // Legacy, ignored with drag threshold
    pub window_type: WindowType,
}
```

## WindowManager

Manages all windows:

```rust
pub struct WindowManager {
    windows: BTreeMap<WindowId, Window>,
    z_order: Vec<WindowId>,
    next_id: WindowId,
    focused: Option<WindowId>,
}

impl WindowManager {
    /// Create a new window
    pub fn create(&mut self, config: WindowConfig) -> WindowId;
    
    /// Close a window
    pub fn close(&mut self, id: WindowId);
    
    /// Get window by ID
    pub fn get(&self, id: WindowId) -> Option<&Window>;
    
    /// Get mutable window
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut Window>;
    
    /// Focus a window (brings to front of z-order)
    pub fn focus(&mut self, id: WindowId);
    
    /// Get currently focused window
    pub fn focused(&self) -> Option<WindowId>;
    
    /// Move a window
    pub fn move_window(&mut self, id: WindowId, position: Vec2);
    
    /// Resize a window
    pub fn resize(&mut self, id: WindowId, size: Size);
    
    /// Minimize a window
    pub fn minimize(&mut self, id: WindowId);
    
    /// Maximize a window to bounds
    pub fn maximize(&mut self, id: WindowId, bounds: Option<Rect>);
    
    /// Restore window to normal state
    pub fn restore(&mut self, id: WindowId);
    
    /// Get all windows
    pub fn all_windows(&self) -> impl Iterator<Item = &Window>;
    
    /// Get windows in z-order (front to back)
    pub fn z_ordered(&self) -> impl Iterator<Item = &Window>;
    
    /// Window count
    pub fn count(&self) -> usize;
}
```

## Window Regions

Windows have distinct regions for hit testing:

```rust
pub enum WindowRegion {
    /// Window title bar (drag to move)
    TitleBar,
    
    /// Window content area
    Content,
    
    /// Close button
    CloseButton,
    
    /// Minimize button
    MinimizeButton,
    
    /// Maximize button
    MaximizeButton,
    
    /// Resize handles
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
}
```

## Hit Testing

```rust
impl Window {
    /// Determine which region a point hits
    pub fn hit_test(&self, point: Vec2) -> Option<WindowRegion> {
        let bounds = self.bounds();
        
        if !bounds.contains(point) {
            return None;
        }
        
        // Check resize handles (outer edges)
        let edge_size = 8.0;
        // ... check edges
        
        // Check title bar
        let title_height = 32.0;
        if point.y < bounds.y + title_height {
            // Check buttons in title bar
            // ...
            return Some(WindowRegion::TitleBar);
        }
        
        Some(WindowRegion::Content)
    }
}
```

## Z-Order

Windows are rendered in z-order (back to front):

```rust
impl WindowManager {
    pub fn focus(&mut self, id: WindowId) {
        // Remove from current position
        self.z_order.retain(|&w| w != id);
        
        // Add to front
        self.z_order.push(id);
        
        self.focused = Some(id);
    }
    
    pub fn z_ordered(&self) -> impl Iterator<Item = &Window> {
        self.z_order.iter()
            .filter_map(|&id| self.windows.get(&id))
    }
}
```

## Process Linking

Windows can be linked to processes for lifecycle management:

```rust
impl DesktopEngine {
    pub fn set_window_process_id(&mut self, id: WindowId, process_id: u64) {
        if let Some(window) = self.windows.get_mut(id) {
            window.process_id = Some(process_id);
            
            // Update title for terminal windows
            if window.app_id == "terminal" {
                window.title = format!("Terminal p{}", process_id);
            }
        }
    }
}
```

When a window is closed, the supervisor can terminate the associated process.

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/window/*.rs`

### Key Invariants
- [ ] Window IDs are unique and monotonic
- [ ] Focused window is always in z-order
- [ ] Minimized windows are not in z-order
- [ ] Restore bounds saved before maximize/minimize
- [ ] Hit test regions don't overlap

### Differences from v0.1.0
- Added process_id linking
- Added WindowType for standard vs widget
- Title bar height is 32px (was variable)
- Resize handle size is 8px
