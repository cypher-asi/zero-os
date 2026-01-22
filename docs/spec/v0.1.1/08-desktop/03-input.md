# Input Routing

## Overview

Input routing determines how mouse events are handled based on what the user clicks. A key feature is the **universal drag threshold** that allows both click-to-interact and drag-to-move on all windows.

## Drag Threshold

The drag threshold is 5 pixels. This enables:

1. **Click**: Mouse down + mouse up within 5px → Click event
2. **Drag**: Mouse down + mouse move > 5px → Drag operation

```rust
const DRAG_THRESHOLD: f32 = 5.0;
```

This applies to all window regions, replacing the legacy `content_interactive` flag.

## Drag State Machine

```rust
pub enum DragState {
    /// Dragging the canvas (pan)
    PanCanvas {
        start: Vec2,
        start_center: Vec2,
    },
    
    /// Moving a window
    MoveWindow {
        window_id: WindowId,
        offset: Vec2,
    },
    
    /// Resizing a window
    ResizeWindow {
        window_id: WindowId,
        handle: WindowRegion,
        start_pos: Vec2,
        start_size: Size,
        start_mouse: Vec2,
    },
}
```

## InputRouter

```rust
pub struct InputRouter {
    drag: Option<DragState>,
}

impl InputRouter {
    pub fn new() -> Self;
    
    /// Get current drag state
    pub fn drag_state(&self) -> Option<&DragState>;
    
    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool;
    
    /// Start canvas pan
    pub fn start_pan(&mut self, start: Vec2, start_center: Vec2);
    
    /// Start window move
    pub fn start_window_move(&mut self, window_id: WindowId, offset: Vec2);
    
    /// Start window resize
    pub fn start_window_resize(
        &mut self,
        window_id: WindowId,
        handle: WindowRegion,
        start_pos: Vec2,
        start_size: Size,
        start_mouse: Vec2,
    );
    
    /// End current drag
    pub fn end_drag(&mut self);
}
```

## InputResult

Returned from input handling to indicate what happened:

```rust
pub enum InputResult {
    /// No window hit - clicked on canvas
    Canvas,
    
    /// Hit window content (for click handling)
    WindowContent(WindowId),
    
    /// Hit window title bar
    WindowTitleBar(WindowId),
    
    /// Hit window button
    WindowButton(WindowId, WindowRegion),
    
    /// Started dragging
    DragStarted,
    
    /// Nothing happened
    None,
}
```

## Mouse Down Flow

```rust
impl DesktopEngine {
    pub fn on_mouse_down(&mut self, x: f32, y: f32, now_ms: f64) -> InputResult {
        // Convert screen to world coordinates
        let world_pos = self.viewport.screen_to_world(Vec2::new(x, y));
        
        // Hit test windows (front to back)
        for window in self.windows.z_ordered().rev() {
            if let Some(region) = window.hit_test(world_pos) {
                match region {
                    WindowRegion::CloseButton => {
                        return InputResult::WindowButton(window.id, region);
                    }
                    WindowRegion::TitleBar | WindowRegion::Content => {
                        // Record potential drag start
                        self.potential_drag = Some(PotentialDrag {
                            window_id: window.id,
                            start_pos: Vec2::new(x, y),
                            region,
                        });
                        self.focus_window(window.id);
                        return InputResult::WindowContent(window.id);
                    }
                    WindowRegion::ResizeSE | ... => {
                        // Start resize immediately
                        self.input.start_window_resize(...);
                        return InputResult::DragStarted;
                    }
                }
            }
        }
        
        // No window hit - start canvas pan
        self.input.start_pan(Vec2::new(x, y), self.viewport.center);
        InputResult::Canvas
    }
}
```

## Mouse Move Flow

```rust
impl DesktopEngine {
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        let pos = Vec2::new(x, y);
        
        // Check if we should start dragging
        if let Some(ref potential) = self.potential_drag {
            let distance = (pos - potential.start_pos).length();
            if distance > DRAG_THRESHOLD {
                // Start actual drag
                match potential.region {
                    WindowRegion::TitleBar | WindowRegion::Content => {
                        let offset = self.calculate_drag_offset(...);
                        self.input.start_window_move(potential.window_id, offset);
                    }
                    _ => {}
                }
                self.potential_drag = None;
            }
            return;
        }
        
        // Handle active drag
        match self.input.drag_state() {
            Some(DragState::PanCanvas { start, start_center }) => {
                let delta = pos - *start;
                self.viewport.center = *start_center - delta / self.viewport.zoom;
            }
            Some(DragState::MoveWindow { window_id, offset }) => {
                let world_pos = self.viewport.screen_to_world(pos);
                self.windows.move_window(*window_id, world_pos - *offset);
            }
            Some(DragState::ResizeWindow { ... }) => {
                // Calculate new size based on mouse position
            }
            None => {}
        }
    }
}
```

## Mouse Up Flow

```rust
impl DesktopEngine {
    pub fn on_mouse_up(&mut self) {
        // If we had a potential drag that never started, it's a click
        if let Some(potential) = self.potential_drag.take() {
            // Handle as click on window
            match potential.region {
                WindowRegion::CloseButton => {
                    self.close_window(potential.window_id);
                }
                WindowRegion::MinimizeButton => {
                    self.minimize_window(potential.window_id);
                }
                WindowRegion::MaximizeButton => {
                    self.toggle_maximize(potential.window_id);
                }
                WindowRegion::Content => {
                    // Click on content - focus only
                }
                _ => {}
            }
        }
        
        // End any active drag
        self.input.end_drag();
    }
}
```

## Scroll/Zoom

```rust
impl DesktopEngine {
    pub fn on_scroll(&mut self, delta_y: f32, x: f32, y: f32) {
        // Zoom centered on mouse position
        let zoom_speed = 0.001;
        let new_zoom = (self.viewport.zoom * (1.0 - delta_y * zoom_speed))
            .clamp(0.1, 5.0);
        
        // Adjust center to zoom toward mouse
        let mouse_world_before = self.viewport.screen_to_world(Vec2::new(x, y));
        self.viewport.zoom = new_zoom;
        let mouse_world_after = self.viewport.screen_to_world(Vec2::new(x, y));
        
        self.viewport.center += mouse_world_before - mouse_world_after;
    }
}
```

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/input/*.rs`
- `crates/zos-desktop/src/engine/input.rs`

### Key Invariants
- [ ] Drag threshold is exactly 5px
- [ ] Click happens only if drag threshold not exceeded
- [ ] Resize handles take priority over content
- [ ] Zoom is clamped to [0.1, 5.0]
- [ ] Pan adjusts for zoom level

### Differences from v0.1.0
- Universal drag threshold replaces content_interactive
- Potential drag state for click/drag detection
- Zoom centered on mouse position
- Resize handles enlarged to 8px
