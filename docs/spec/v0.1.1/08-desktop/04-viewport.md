# Viewport and Camera

## Overview

The viewport manages the view into the infinite desktop canvas. It handles coordinate transformations between screen space and world space.

## Viewport Structure

```rust
pub struct Viewport {
    /// Screen size in pixels
    pub screen_size: Size,
    
    /// Camera center in world coordinates
    pub center: Vec2,
    
    /// Zoom level (1.0 = 100%, 0.5 = 50%, 2.0 = 200%)
    pub zoom: f32,
}
```

## Camera Structure

```rust
pub struct Camera {
    /// Center position in world coordinates
    pub center: Vec2,
    
    /// Zoom level
    pub zoom: f32,
}

impl Camera {
    pub fn at(center: Vec2, zoom: f32) -> Self {
        Self { center, zoom }
    }
}
```

## Coordinate Transformation

### Screen to World

Convert screen coordinates (pixels) to world coordinates (canvas position):

```rust
impl Viewport {
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        // Screen center
        let screen_center = Vec2::new(
            self.screen_size.width / 2.0,
            self.screen_size.height / 2.0,
        );
        
        // Offset from screen center
        let offset = screen_pos - screen_center;
        
        // Apply zoom and translate
        self.center + offset / self.zoom
    }
}
```

### World to Screen

Convert world coordinates to screen coordinates:

```rust
impl Viewport {
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let screen_center = Vec2::new(
            self.screen_size.width / 2.0,
            self.screen_size.height / 2.0,
        );
        
        let offset = world_pos - self.center;
        
        screen_center + offset * self.zoom
    }
}
```

## Visible Rect

Get the visible area in world coordinates:

```rust
impl Viewport {
    pub fn visible_rect(&self) -> Rect {
        let half_w = self.screen_size.width / (2.0 * self.zoom);
        let half_h = self.screen_size.height / (2.0 * self.zoom);
        
        Rect::new(
            self.center.x - half_w,
            self.center.y - half_h,
            half_w * 2.0,
            half_h * 2.0,
        )
    }
}
```

## Camera Animations

Camera transitions are animated using easing functions:

```rust
pub struct CameraAnimation {
    /// Start camera position
    pub start: Camera,
    
    /// Target camera position
    pub target: Camera,
    
    /// Animation start time (ms)
    pub start_time: f64,
    
    /// Animation duration (ms)
    pub duration: f64,
}

impl CameraAnimation {
    pub fn update(&self, now_ms: f64) -> Camera {
        let t = ((now_ms - self.start_time) / self.duration).clamp(0.0, 1.0);
        let eased = ease_out_cubic(t);
        
        Camera {
            center: self.start.center.lerp(self.target.center, eased),
            zoom: self.start.zoom + (self.target.zoom - self.start.zoom) * eased,
        }
    }
    
    pub fn is_complete(&self, now_ms: f64) -> bool {
        now_ms >= self.start_time + self.duration
    }
}
```

### Animation Duration

```rust
pub const CAMERA_ANIMATION_DURATION_MS: f64 = 300.0;
```

## Per-Window Camera

Each window can have a saved camera position for "focus and center":

```rust
impl DesktopEngine {
    pub fn focus_window(&mut self, id: WindowId) {
        // Save current camera for previously focused window
        if let Some(prev_id) = self.windows.focused() {
            if prev_id != id {
                self.window_cameras.insert(
                    prev_id, 
                    Camera::at(self.viewport.center, self.viewport.zoom)
                );
            }
        }
        
        self.windows.focus(id);
    }
}
```

## Per-Desktop Camera

Each desktop saves its camera position when switching:

```rust
impl DesktopManager {
    pub fn save_desktop_camera(&mut self, index: usize, center: Vec2, zoom: f32) {
        if let Some(desktop) = self.desktops.get_mut(index) {
            desktop.camera = Some(Camera::at(center, zoom));
        }
    }
    
    pub fn get_desktop_camera(&self, index: usize) -> Option<Camera> {
        self.desktops.get(index).and_then(|d| d.camera)
    }
}
```

## Zoom Limits

```rust
const MIN_ZOOM: f32 = 0.1;  // 10%
const MAX_ZOOM: f32 = 5.0;  // 500%
```

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/viewport.rs`
- `crates/zos-desktop/src/math/camera.rs`
- `crates/zos-desktop/src/transition/camera.rs`

### Key Invariants
- [ ] Zoom is clamped to [0.1, 5.0]
- [ ] Screen center maps to viewport center in world space
- [ ] Camera animation duration is 300ms
- [ ] Animations use ease-out-cubic easing

### Differences from v0.1.0
- Per-window camera positions saved
- Per-desktop camera positions saved
- Visible rect calculation accounts for zoom
- Animation uses easing functions
