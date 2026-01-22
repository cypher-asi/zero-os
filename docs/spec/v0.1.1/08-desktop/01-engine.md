# Desktop Engine

## Overview

`DesktopEngine` is the central coordinator for the desktop environment. It manages windows, desktops, input, and animations.

## Structure

```rust
pub struct DesktopEngine {
    // Core managers
    windows: WindowManager,
    desktops: DesktopManager,
    input: InputRouter,
    
    // Viewport
    viewport: Viewport,
    
    // Animations
    crossfade: Option<Crossfade>,
    camera_animation: Option<CameraAnimation>,
    
    // Per-window camera positions
    window_cameras: HashMap<WindowId, Camera>,
}
```

## Initialization

```rust
impl DesktopEngine {
    pub fn new() -> Self {
        Self {
            windows: WindowManager::new(),
            desktops: DesktopManager::new(),
            input: InputRouter::new(),
            viewport: Viewport::default(),
            crossfade: None,
            camera_animation: None,
            window_cameras: HashMap::new(),
        }
    }

    pub fn init(&mut self, screen_width: f32, screen_height: f32) {
        self.viewport.screen_size = Size::new(screen_width, screen_height);
        self.viewport.center = Vec2::ZERO;
        self.viewport.zoom = 1.0;
        
        // Create default desktop
        self.desktops.create("Desktop 1");
    }
}
```

## Window Operations

```rust
impl DesktopEngine {
    /// Create a window
    pub fn create_window(&mut self, config: WindowConfig) -> WindowId;
    
    /// Close a window
    pub fn close_window(&mut self, id: WindowId);
    
    /// Focus a window (brings to front)
    pub fn focus_window(&mut self, id: WindowId);
    
    /// Move a window
    pub fn move_window(&mut self, id: WindowId, x: f32, y: f32);
    
    /// Resize a window
    pub fn resize_window(&mut self, id: WindowId, width: f32, height: f32);
    
    /// Minimize a window
    pub fn minimize_window(&mut self, id: WindowId);
    
    /// Maximize a window
    pub fn maximize_window(&mut self, id: WindowId);
    
    /// Restore a minimized/maximized window
    pub fn restore_window(&mut self, id: WindowId);
    
    /// Launch an application (creates window with app_id)
    pub fn launch_app(&mut self, app_id: &str) -> WindowId;
    
    /// Set process ID for a window
    pub fn set_window_process_id(&mut self, id: WindowId, process_id: u64);
    
    /// Get process ID for a window
    pub fn get_window_process_id(&self, id: WindowId) -> Option<u64>;
}
```

## Desktop Operations

```rust
impl DesktopEngine {
    /// Create a new desktop
    pub fn create_desktop(&mut self, name: &str) -> DesktopId;
    
    /// Switch to desktop by index
    pub fn switch_desktop(&mut self, index: usize, now_ms: f64);
    
    /// Enter void (zoomed-out overview)
    pub fn enter_void(&mut self, now_ms: f64);
    
    /// Exit void to active desktop
    pub fn exit_void(&mut self, now_ms: f64);
    
    /// Set desktop background
    pub fn set_desktop_background(&mut self, index: usize, background: &str);
}
```

## Input Handling

```rust
impl DesktopEngine {
    /// Handle mouse down event
    /// Returns InputResult indicating what was hit
    pub fn on_mouse_down(&mut self, x: f32, y: f32, now_ms: f64) -> InputResult;
    
    /// Handle mouse move event
    pub fn on_mouse_move(&mut self, x: f32, y: f32);
    
    /// Handle mouse up event
    pub fn on_mouse_up(&mut self);
    
    /// Handle scroll/zoom event
    pub fn on_scroll(&mut self, delta_y: f32, x: f32, y: f32);
}
```

## Animation Updates

```rust
impl DesktopEngine {
    /// Update animations (call each frame)
    pub fn update(&mut self, now_ms: f64);
    
    /// Check if any animations are in progress
    pub fn is_animating(&self) -> bool;
    
    /// Check if crossfading between desktops
    pub fn is_crossfading(&self) -> bool;
}
```

## Render State

The engine provides serializable render state for React:

```rust
impl DesktopEngine {
    /// Get render state as JSON
    pub fn render_state(&self) -> String;
    
    /// Get window screen rectangles (for React rendering)
    pub fn get_window_rects(&self) -> Vec<WindowScreenRect>;
}

pub struct WindowScreenRect {
    pub id: WindowId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub z_index: i32,
    pub app_id: String,
    pub title: String,
    pub focused: bool,
    pub minimized: bool,
    pub maximized: bool,
}
```

## Cascade Positioning

New windows are positioned using cascade logic:

```rust
fn calculate_cascade_position(&self, config: &WindowConfig) -> Vec2 {
    let cascade_offset = 50.0;
    
    // Get the most recently created window
    let last_window = self.windows.all_windows()
        .max_by_key(|w| w.id);
    
    if let Some(last) = last_window {
        Vec2::new(
            last.position.x + cascade_offset,
            last.position.y + cascade_offset,
        )
    } else {
        // Center first window
        Vec2::new(
            -config.size.width / 2.0,
            -config.size.height / 2.0,
        )
    }
}
```

## App Configurations

Built-in app configurations:

| App ID | Window Type | Size | Notes |
|--------|-------------|------|-------|
| terminal | Standard | 900×600 | Title shows PID |
| clock | Widget | 280×280 | Smaller, no resize handles |
| calculator | Widget | 320×450 | Fixed aspect ratio |
| browser | Standard | 900×600 | Generic app |

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/engine/*.rs`

### Key Invariants
- [ ] Window IDs are never reused
- [ ] Focused window is always in z-order
- [ ] Cascade offset is 50px
- [ ] Animations complete before new ones start

### Differences from v0.1.0
- Process ID linking for window lifecycle
- Per-window camera position saving
- Widget window type with smaller size
- App configurations for standard apps
