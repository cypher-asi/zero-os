# Input Handling

**Component:** 10-desktop/03-input  
**Status:** Specification

---

## Overview

Input handling routes events to the engine (for canvas/window manipulation) or to React (for UI content).

---

## Input Events

```rust
/// Input event
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Pointer (mouse/touch) events
    Pointer(PointerEvent),
    
    /// Keyboard events
    Keyboard(KeyboardEvent),
    
    /// Scroll/wheel events
    Scroll(ScrollEvent),
    
    /// Touch gesture events
    Gesture(GestureEvent),
}

/// Pointer event
#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub position: Vec2,
    pub button: Option<PointerButton>,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug)]
pub enum PointerKind {
    Down,
    Up,
    Move,
    Enter,
    Leave,
}

#[derive(Clone, Copy, Debug)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

/// Keyboard event
#[derive(Clone, Debug)]
pub struct KeyboardEvent {
    pub kind: KeyKind,
    pub key: Key,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug)]
pub enum KeyKind {
    Down,
    Up,
}

/// Scroll event
#[derive(Clone, Debug)]
pub struct ScrollEvent {
    pub delta: Vec2,
    pub position: Vec2,
    pub modifiers: Modifiers,
}

/// Gesture event
#[derive(Clone, Debug)]
pub struct GestureEvent {
    pub kind: GestureKind,
    pub center: Vec2,
    pub delta: Vec2,
    pub scale: f32,
    pub rotation: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum GestureKind {
    Pan,
    Pinch,
    Rotate,
}

/// Modifier keys
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}
```

---

## Input Router

```rust
/// Input router
pub struct InputRouter {
    /// Desktop engine
    engine: DesktopEngine,
    
    /// Window manager
    windows: WindowManager,
    
    /// Current drag operation
    drag: Option<DragState>,
    
    /// Focused window
    focused: Option<WindowId>,
}

impl InputRouter {
    /// Handle input event
    pub fn handle(&mut self, event: InputEvent) -> InputResult {
        match event {
            InputEvent::Pointer(pointer) => self.handle_pointer(pointer),
            InputEvent::Keyboard(keyboard) => self.handle_keyboard(keyboard),
            InputEvent::Scroll(scroll) => self.handle_scroll(scroll),
            InputEvent::Gesture(gesture) => self.handle_gesture(gesture),
        }
    }
    
    fn handle_pointer(&mut self, event: PointerEvent) -> InputResult {
        match event.kind {
            PointerKind::Down => {
                // Check what was clicked
                if let Some((window_id, region)) = 
                    self.windows.window_region_at(event.position, &self.engine.viewport) 
                {
                    match region {
                        WindowRegion::TitleBar => {
                            // Start window drag
                            self.start_drag(DragKind::MoveWindow(window_id), event.position);
                            self.windows.focus_window(window_id);
                            InputResult::Handled
                        }
                        WindowRegion::Content => {
                            // Forward to window content (React)
                            self.windows.focus_window(window_id);
                            InputResult::ForwardToWindow(window_id, event)
                        }
                        WindowRegion::CloseButton => {
                            self.windows.close_window(window_id);
                            InputResult::Handled
                        }
                        WindowRegion::ResizeBottomRight => {
                            self.start_drag(DragKind::ResizeWindow(window_id), event.position);
                            InputResult::Handled
                        }
                        _ => InputResult::Handled,
                    }
                } else {
                    // Click on canvas - start pan
                    self.start_drag(DragKind::PanCanvas, event.position);
                    InputResult::Handled
                }
            }
            PointerKind::Move => {
                if let Some(drag) = &mut self.drag {
                    let delta = event.position - drag.last_position;
                    drag.last_position = event.position;
                    
                    match drag.kind {
                        DragKind::PanCanvas => {
                            self.engine.pan(delta);
                        }
                        DragKind::MoveWindow(id) => {
                            let canvas_delta = delta / self.engine.viewport.zoom;
                            let window = self.windows.windows.get(&id).unwrap();
                            self.windows.move_window(id, window.position + canvas_delta);
                        }
                        DragKind::ResizeWindow(id) => {
                            let canvas_delta = delta / self.engine.viewport.zoom;
                            let window = self.windows.windows.get(&id).unwrap();
                            let new_size = Size::new(
                                window.size.width + canvas_delta.x,
                                window.size.height + canvas_delta.y,
                            );
                            self.windows.resize_window(id, new_size);
                        }
                    }
                    InputResult::Handled
                } else if let Some((window_id, WindowRegion::Content)) = 
                    self.windows.window_region_at(event.position, &self.engine.viewport) 
                {
                    // Forward hover to window content
                    InputResult::ForwardToWindow(window_id, event)
                } else {
                    InputResult::Unhandled
                }
            }
            PointerKind::Up => {
                self.drag = None;
                InputResult::Handled
            }
            _ => InputResult::Unhandled,
        }
    }
    
    fn handle_scroll(&mut self, event: ScrollEvent) -> InputResult {
        if event.modifiers.ctrl {
            // Ctrl+scroll = zoom
            let factor = 1.0 + event.delta.y * 0.1;
            self.engine.zoom(factor, event.position);
        } else {
            // Regular scroll = pan
            self.engine.pan(-event.delta);
        }
        InputResult::Handled
    }
    
    fn handle_gesture(&mut self, event: GestureEvent) -> InputResult {
        match event.kind {
            GestureKind::Pan => {
                self.engine.pan(event.delta);
            }
            GestureKind::Pinch => {
                self.engine.zoom(event.scale, event.center);
            }
            _ => {}
        }
        InputResult::Handled
    }
    
    fn handle_keyboard(&mut self, event: KeyboardEvent) -> InputResult {
        // Forward to focused window
        if let Some(window_id) = self.focused {
            InputResult::ForwardToWindow(window_id, InputEvent::Keyboard(event))
        } else {
            InputResult::Unhandled
        }
    }
    
    fn start_drag(&mut self, kind: DragKind, position: Vec2) {
        self.drag = Some(DragState {
            kind,
            start_position: position,
            last_position: position,
        });
    }
}

/// Drag state
struct DragState {
    kind: DragKind,
    start_position: Vec2,
    last_position: Vec2,
}

/// Drag operation kind
enum DragKind {
    PanCanvas,
    MoveWindow(WindowId),
    ResizeWindow(WindowId),
}

/// Input handling result
pub enum InputResult {
    Handled,
    Unhandled,
    ForwardToWindow(WindowId, impl Into<InputEvent>),
}
```

---

*[Back to Desktop](README.md) | [Previous: Windows](02-windows.md) | [Next: Presentation](04-presentation.md)*
