//! Desktop engine coordinating all components

use std::collections::HashMap;
use crate::desktop::{DesktopId, DesktopManager, VoidState};
use crate::input::{DragState, InputResult, InputRouter};
use crate::math::{Camera, Rect, Size, Vec2};
use crate::transition::{CameraAnimation, Crossfade, CrossfadeDirection};
use crate::viewport::Viewport;
use crate::view_mode::ViewMode;
use crate::window::{WindowConfig, WindowId, WindowManager, WindowRegion, WindowState};

/// Window with screen-space coordinates for rendering
#[derive(Clone, Debug)]
pub struct WindowScreenRect {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub state: WindowState,
    pub focused: bool,
    pub screen_rect: Rect,
    /// Opacity for fade transitions (0.0 = invisible, 1.0 = fully visible)
    pub opacity: f32,
    /// Whether the window content area handles its own mouse events
    pub content_interactive: bool,
}

/// Desktop engine coordinating all desktop components
///
/// This is the main entry point for desktop operations, managing:
/// - View mode (desktop or void)
/// - Layer cameras (each desktop has a camera, void has its own)
/// - Window manager (window CRUD, focus, z-order)
/// - Desktop manager (separate infinite canvases)
/// - Input router (drag/resize state machine)
/// - Crossfade transitions (opacity animations between layers)
pub struct DesktopEngine {
    /// Current view mode (desktop or void)
    pub view_mode: ViewMode,
    /// Void layer state
    pub void_state: VoidState,
    /// Legacy viewport (for backward compatibility)
    pub viewport: Viewport,
    /// Window manager
    pub windows: WindowManager,
    /// Desktop manager
    pub desktops: DesktopManager,
    /// Input router
    pub input: InputRouter,
    /// Current crossfade transition
    crossfade: Option<Crossfade>,
    /// Camera animation
    camera_animation: Option<CameraAnimation>,
    /// Last viewport activity time (ms) for animation detection
    last_activity_ms: f64,
    /// Per-window camera memory (remembers camera position for each window)
    window_cameras: HashMap<WindowId, Camera>,
}

impl Default for DesktopEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopEngine {
    /// Create a new desktop engine
    pub fn new() -> Self {
        Self {
            view_mode: ViewMode::default(),
            void_state: VoidState::default(),
            viewport: Viewport::default(),
            windows: WindowManager::new(),
            desktops: DesktopManager::new(),
            input: InputRouter::new(),
            crossfade: None,
            camera_animation: None,
            last_activity_ms: 0.0,
            window_cameras: HashMap::new(),
        }
    }

    /// Initialize the desktop with screen dimensions
    pub fn init(&mut self, width: f32, height: f32) {
        // #region agent log
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" { fn fetch(url: &str, options: &JsValue) -> js_sys::Promise; }
        let log_data_before = format!("{{\"location\":\"engine.rs:81\",\"message\":\"init - before\",\"data\":{{\"width\":{},\"height\":{},\"desktopCount\":{},\"viewportCenterX\":{},\"viewportCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"A1\"}}", width, height, self.desktops.desktops().len(), self.viewport.center.x, self.viewport.center.y, js_sys::Date::now() as u64);
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &"method".into(), &"POST".into()).ok();
        let headers = js_sys::Object::new();
        js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts, &"headers".into(), &headers).ok();
        js_sys::Reflect::set(&opts, &"body".into(), &log_data_before.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts);
        // #endregion
        
        let screen_size = Size::new(width, height);
        self.viewport.screen_size = screen_size;
        self.void_state.set_screen_size(screen_size);
        self.desktops.set_desktop_size(Size::new(width.max(1920.0), height.max(1080.0)));

        // Only create first desktop if none exists yet (idempotent for React StrictMode)
        if self.desktops.desktops().is_empty() {
            let desktop_id = self.desktops.create("Main");
            if let Some(desktop) = self.desktops.get(desktop_id) {
                self.viewport.center = desktop.bounds.center();
                
                // #region agent log
                let log_data_after = format!("{{\"location\":\"engine.rs:89\",\"message\":\"init - after create desktop\",\"data\":{{\"desktopId\":{},\"boundsX\":{},\"boundsY\":{},\"boundsW\":{},\"boundsH\":{},\"centerX\":{},\"centerY\":{},\"viewportCenterX\":{},\"viewportCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"A1,A2\"}}", desktop_id, desktop.bounds.x, desktop.bounds.y, desktop.bounds.width, desktop.bounds.height, desktop.bounds.center().x, desktop.bounds.center().y, self.viewport.center.x, self.viewport.center.y, js_sys::Date::now() as u64);
                let opts2 = js_sys::Object::new();
                js_sys::Reflect::set(&opts2, &"method".into(), &"POST".into()).ok();
                let headers2 = js_sys::Object::new();
                js_sys::Reflect::set(&headers2, &"Content-Type".into(), &"application/json".into()).ok();
                js_sys::Reflect::set(&opts2, &"headers".into(), &headers2).ok();
                js_sys::Reflect::set(&opts2, &"body".into(), &log_data_after.into()).ok();
                let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts2);
                // #endregion
            }
        }
    }

    /// Resize the viewport
    pub fn resize(&mut self, width: f32, height: f32) {
        let screen_size = Size::new(width, height);
        self.viewport.screen_size = screen_size;
        self.void_state.set_screen_size(screen_size);

        let min_width = width.max(1920.0);
        let min_height = height.max(1080.0);
        self.desktops.set_desktop_size(Size::new(min_width, min_height));
    }

    /// Get the current crossfade transition
    pub fn crossfade(&self) -> Option<&Crossfade> {
        self.crossfade.as_ref()
    }

    /// Check if a crossfade is active
    pub fn is_crossfading(&self) -> bool {
        self.crossfade.is_some()
    }

    /// Check if any transition is active
    pub fn is_transitioning(&self) -> bool {
        self.is_crossfading() || self.camera_animation.is_some()
    }

    /// Check if void layer should be visible (during void mode or void transitions)
    pub fn should_show_void(&self) -> bool {
        match &self.view_mode {
            ViewMode::Void => true,
            ViewMode::Desktop { .. } => {
                // Only show void during transitions TO or FROM void, not during desktop switches
                if let Some(ref crossfade) = self.crossfade {
                    matches!(
                        crossfade.direction,
                        CrossfadeDirection::ToVoid | CrossfadeDirection::ToDesktop
                    )
                } else {
                    false
                }
            }
        }
    }

    /// Get layer opacities (desktop_opacity, void_opacity)
    pub fn layer_opacities(&self, now_ms: f64) -> (f32, f32) {
        if let Some(ref crossfade) = self.crossfade {
            crossfade.opacities(now_ms)
        } else {
            match self.view_mode {
                ViewMode::Desktop { .. } => (1.0, 0.0),
                ViewMode::Void => (0.0, 1.0),
            }
        }
    }

    /// Get the current view mode
    pub fn get_view_mode(&self) -> &ViewMode {
        &self.view_mode
    }

    /// Check if in void mode
    pub fn is_in_void(&self) -> bool {
        self.view_mode.is_void()
    }

    /// Pan the viewport
    pub fn pan(&mut self, dx: f32, dy: f32) {
        if self.is_crossfading() {
            return;
        }

        match &self.view_mode {
            ViewMode::Desktop { .. } => {
                self.viewport.pan(dx, dy);
                self.commit_viewport_to_desktop();
            }
            ViewMode::Void => {
                self.viewport.pan(dx, dy);
                let bounds: Vec<Rect> = self.desktops.desktops().iter().map(|d| d.bounds).collect();
                self.void_state.constrain_to_desktops(&bounds, 200.0);
            }
        }
    }

    /// Zoom at anchor point
    pub fn zoom_at(&mut self, factor: f32, anchor_x: f32, anchor_y: f32) {
        if self.is_crossfading() {
            return;
        }

        match &self.view_mode {
            ViewMode::Desktop { .. } => {
                self.viewport.zoom_at(factor, anchor_x, anchor_y);
                if self.viewport.zoom < 0.001 {
                    self.viewport.zoom = 0.001;
                }
                self.commit_viewport_to_desktop();
            }
            ViewMode::Void => {
                self.viewport.zoom_at_clamped(factor, anchor_x, anchor_y, 0.1, 1.0);
            }
        }
    }

    /// Commit viewport state to active desktop
    fn commit_viewport_to_desktop(&mut self) {
        if let ViewMode::Desktop { index } = self.view_mode {
            self.desktops.save_desktop_camera(index, self.viewport.center, self.viewport.zoom);
        }
    }

    /// Create a window
    pub fn create_window(&mut self, mut config: WindowConfig) -> WindowId {
        // If no position specified, cascade from most recent window or center on viewport
        if config.position.is_none() {
            let cascade_offset = 50.0;  // Cascade offset for diagonal window stacking
            let max_cascade = 5.0;
            
            // Get the most recently CREATED window to cascade from (highest window ID)
            // Note: windows_by_z() returns by z-order (focus), not creation order!
            let last_window_pos = self.windows.all_windows()
                .max_by_key(|w| w.id)
                .map(|w| w.position);
            
            let position = if let Some(last_pos) = last_window_pos {
                // Cascade from the last window's position
                let _window_count = self.windows.count() as f32;
                let _cascade_index = (_window_count % max_cascade).max(1.0);
                
                // Cascade diagonally from last window
                Vec2::new(
                    last_pos.x + cascade_offset,
                    last_pos.y + cascade_offset,
                )
            } else {
                // First window - center on desktop canvas origin (0, 0)
                // Use Vec2::ZERO instead of viewport.center so the position is consistent
                // regardless of where the user has panned/zoomed the viewport
                let desktop_center = Vec2::ZERO;
                let half_w = config.size.width / 2.0;
                let half_h = config.size.height / 2.0;
                Vec2::new(
                    desktop_center.x - half_w,
                    desktop_center.y - half_h,
                )
            };
            
            config.position = Some(position);
        }
        
        let id = self.windows.create(config);
        let active = self.desktops.active_index();
        self.desktops.add_window_to_desktop(active, id);
        
        id
    }

    /// Close a window
    pub fn close_window(&mut self, id: WindowId) {
        // #region agent log
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" { fn fetch(url: &str, options: &JsValue) -> js_sys::Promise; }
        let log_data_before = format!("{{\"location\":\"engine.rs:231\",\"message\":\"close_window - before close\",\"data\":{{\"windowId\":{},\"viewportCenterX\":{},\"viewportCenterY\":{},\"viewportZoom\":{},\"hasCameraAnimation\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"B1,B2\"}}", id, self.viewport.center.x, self.viewport.center.y, self.viewport.zoom, self.camera_animation.is_some(), js_sys::Date::now() as u64);
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &"method".into(), &"POST".into()).ok();
        let headers = js_sys::Object::new();
        js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts, &"headers".into(), &headers).ok();
        js_sys::Reflect::set(&opts, &"body".into(), &log_data_before.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts);
        // #endregion
        
        // Cancel any camera animation when closing a window to prevent unwanted panning
        self.camera_animation = None;
        
        self.desktops.remove_window(id);
        self.windows.close(id);
        // Clean up saved camera position for this window
        self.window_cameras.remove(&id);
        
        // #region agent log
        let log_data_after = format!("{{\"location\":\"engine.rs:239\",\"message\":\"close_window - after close\",\"data\":{{\"windowId\":{},\"viewportCenterX\":{},\"viewportCenterY\":{},\"viewportZoom\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"B1,B2\"}}", id, self.viewport.center.x, self.viewport.center.y, self.viewport.zoom, js_sys::Date::now() as u64);
        let opts2 = js_sys::Object::new();
        js_sys::Reflect::set(&opts2, &"method".into(), &"POST".into()).ok();
        let headers2 = js_sys::Object::new();
        js_sys::Reflect::set(&headers2, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts2, &"headers".into(), &headers2).ok();
        js_sys::Reflect::set(&opts2, &"body".into(), &log_data_after.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts2);
        // #endregion
    }

    /// Get the process ID for a window (if any)
    pub fn get_window_process_id(&self, id: WindowId) -> Option<u64> {
        self.windows.get(id).and_then(|w| w.process_id)
    }

    /// Focus a window
    pub fn focus_window(&mut self, id: WindowId) {
        // #region agent log
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" { fn fetch(url: &str, options: &JsValue) -> js_sys::Promise; }
        let log_before = format!("{{\"location\":\"engine.rs:367\",\"message\":\"focus_window - before\",\"data\":{{\"windowId\":{},\"viewportCenterX\":{},\"viewportCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"FOCUS\"}}", id, self.viewport.center.x, self.viewport.center.y, js_sys::Date::now() as u64);
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &"method".into(), &"POST".into()).ok();
        let headers = js_sys::Object::new();
        js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts, &"headers".into(), &headers).ok();
        js_sys::Reflect::set(&opts, &"body".into(), &log_before.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts);
        // #endregion
        
        // Save current camera position for the previously focused window
        if let Some(prev_id) = self.windows.focused() {
            if prev_id != id {
                self.window_cameras.insert(prev_id, Camera::at(self.viewport.center, self.viewport.zoom));
            }
        }
        
        self.windows.focus(id);
        
        // #region agent log
        let log_after = format!("{{\"location\":\"engine.rs:375\",\"message\":\"focus_window - after\",\"data\":{{\"windowId\":{},\"viewportCenterX\":{},\"viewportCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"FOCUS\"}}", id, self.viewport.center.x, self.viewport.center.y, js_sys::Date::now() as u64);
        let opts2 = js_sys::Object::new();
        js_sys::Reflect::set(&opts2, &"method".into(), &"POST".into()).ok();
        let headers2 = js_sys::Object::new();
        js_sys::Reflect::set(&headers2, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts2, &"headers".into(), &headers2).ok();
        js_sys::Reflect::set(&opts2, &"body".into(), &log_after.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts2);
        // #endregion
    }

    /// Move a window
    pub fn move_window(&mut self, id: WindowId, x: f32, y: f32) {
        self.windows.move_window(id, Vec2::new(x, y));
    }

    /// Resize a window
    pub fn resize_window(&mut self, id: WindowId, width: f32, height: f32) {
        self.windows.resize(id, Size::new(width, height));
    }

    /// Minimize a window
    pub fn minimize_window(&mut self, id: WindowId) {
        self.windows.minimize(id);
    }

    /// Maximize a window
    pub fn maximize_window(&mut self, id: WindowId) {
        let taskbar_height = 48.0;
        
        // Get the visible canvas area considering current camera position and zoom
        let visible = self.viewport.visible_rect();
        
        // Adjust for taskbar at bottom of screen (taskbar height is in screen pixels, so scale by zoom)
        let maximize_bounds = Rect::new(
            visible.x,
            visible.y,
            visible.width,
            visible.height - taskbar_height / self.viewport.zoom,
        );
        self.windows.maximize(id, Some(maximize_bounds));
    }

    /// Restore a window
    pub fn restore_window(&mut self, id: WindowId) {
        self.windows.restore(id);
    }

    /// Create a desktop
    pub fn create_desktop(&mut self, name: &str) -> DesktopId {
        self.desktops.create(name)
    }

    /// Set background for a desktop by index
    pub fn set_desktop_background(&mut self, desktop_index: usize, background: &str) {
        self.desktops.set_desktop_background(desktop_index, background);
    }

    /// Switch to desktop by index
    pub fn switch_desktop(&mut self, index: usize, now_ms: f64) {
        // Don't allow switching during drag operations
        if self.input.is_dragging() {
            return;
        }
        
        // Block if transitioning to/from void (preserve those animations)
        // but allow interrupting desktop-to-desktop switches for responsive navigation
        if let Some(ref crossfade) = self.crossfade {
            match crossfade.direction {
                CrossfadeDirection::ToVoid | CrossfadeDirection::ToDesktop => {
                    // Don't interrupt void transitions
                    return;
                }
                CrossfadeDirection::SwitchDesktop => {
                    // Allow interrupting desktop switches for responsive multi-step navigation
                    // (e.g., Ctrl+Arrow, Arrow to move 2 desktops quickly)
                }
            }
        }

        let current_index = self.desktops.active_index();
        if current_index == index {
            return;
        }

        self.desktops.save_desktop_camera(current_index, self.viewport.center, self.viewport.zoom);

        if self.desktops.switch_to(index) {
            self.focus_top_window_on_desktop(index);
            self.crossfade = Some(Crossfade::switch_desktop(now_ms, current_index, index));

            // NOTE: Viewport will be updated when crossfade completes in tick_transition()
            // Updating it here causes windows to jump before the transition begins
        }
    }

    /// Focus top window on a desktop
    fn focus_top_window_on_desktop(&mut self, desktop_index: usize) {
        let desktop = match self.desktops.desktops().get(desktop_index) {
            Some(d) => d,
            None => return,
        };

        let top_window = self
            .windows
            .windows_by_z()
            .into_iter()
            .rfind(|w| desktop.contains_window(w.id) && w.state != WindowState::Minimized);

        if let Some(window) = top_window {
            self.windows.focus(window.id);
        }
    }

    /// Tick transitions, returns true if any transition is active
    pub fn tick_transition(&mut self, now_ms: f64) -> bool {
        // Tick crossfade
        if let Some(ref crossfade) = self.crossfade {
            // For desktop switches: sync viewport to match the currently visible desktop
            // This ensures windows always render with the correct viewport during crossfade
            if matches!(crossfade.direction, CrossfadeDirection::SwitchDesktop) {
                let visual_workspace = self.get_visual_active_workspace_at(now_ms);
                
                // Sync viewport to match the currently visible desktop's camera
                if let Some(saved) = self.desktops.get_desktop_camera(visual_workspace) {
                    self.viewport.center = saved.center;
                    self.viewport.zoom = saved.zoom;
                }
            }
            
            if crossfade.is_complete(now_ms) {
                match crossfade.direction {
                    CrossfadeDirection::ToVoid => {
                        self.view_mode = ViewMode::Void;
                        self.viewport.center = self.void_state.camera().center;
                        self.viewport.zoom = self.void_state.camera().zoom;
                    }
                    CrossfadeDirection::ToDesktop | CrossfadeDirection::SwitchDesktop => {
                        let index = crossfade.target_desktop.unwrap_or(0);
                        self.view_mode = ViewMode::Desktop { index };
                        if let Some(saved) = self.desktops.get_desktop_camera(index) {
                            self.viewport.center = saved.center;
                            self.viewport.zoom = saved.zoom;
                        }
                        self.focus_top_window_on_desktop(index);
                    }
                }
                self.crossfade = None;
                return self.camera_animation.is_some();
            }
        }

        // Tick camera animation
        if let Some(ref animation) = self.camera_animation {
            if animation.is_complete(now_ms) {
                let final_camera = animation.final_camera();
                self.viewport.center = final_camera.center;
                self.viewport.zoom = final_camera.zoom;
                self.camera_animation = None;
                return self.is_crossfading();
            } else {
                let current = animation.current(now_ms);
                self.viewport.center = current.center;
                self.viewport.zoom = current.zoom;
                return true;
            }
        }

        self.is_crossfading()
    }

    /// Get active camera
    pub fn active_camera(&self) -> Camera {
        match self.view_mode {
            ViewMode::Desktop { index } => self
                .desktops
                .desktops()
                .get(index)
                .map(|d| d.camera())
                .unwrap_or_default(),
            ViewMode::Void => *self.void_state.camera(),
        }
    }

    /// Start move drag
    pub fn start_move_drag(&mut self, id: WindowId, screen_x: f32, screen_y: f32) {
        self.camera_animation = None;

        let window_position = match self.windows.get(id) {
            Some(window) => window.position,
            None => return,
        };

        let canvas_pos = self.viewport.screen_to_canvas(Vec2::new(screen_x, screen_y));
        let offset = canvas_pos - window_position;
        self.windows.focus(id);
        self.input.start_window_move(id, offset);
    }

    /// Start resize drag
    pub fn start_resize_drag(&mut self, id: WindowId, direction: &str, screen_x: f32, screen_y: f32) {
        self.camera_animation = None;

        let handle = match direction {
            "n" => WindowRegion::ResizeN,
            "s" => WindowRegion::ResizeS,
            "e" => WindowRegion::ResizeE,
            "w" => WindowRegion::ResizeW,
            "ne" => WindowRegion::ResizeNE,
            "nw" => WindowRegion::ResizeNW,
            "se" => WindowRegion::ResizeSE,
            "sw" => WindowRegion::ResizeSW,
            _ => return,
        };

        if let Some(window) = self.windows.get(id) {
            let canvas_pos = self.viewport.screen_to_canvas(Vec2::new(screen_x, screen_y));
            self.input.start_window_resize(id, handle, window.position, window.size, canvas_pos);
        }
    }

    /// Handle pointer down
    pub fn handle_pointer_down(&mut self, x: f32, y: f32, button: u8, ctrl: bool, shift: bool) -> InputResult {
        let screen_pos = Vec2::new(x, y);
        let canvas_pos = self.viewport.screen_to_canvas(screen_pos);

        // Middle mouse or ctrl/shift + left = pan
        if button == 1 || (button == 0 && (ctrl || shift)) {
            self.camera_animation = None;
            self.input.start_pan(screen_pos, self.viewport.center);
            return InputResult::Handled;
        }

        // Left button - check windows
        if button == 0 {
            let active_windows = &self.desktops.active_desktop().windows;
            let zoom = self.viewport.zoom;

            if let Some((window_id, region)) = self.windows.region_at_filtered(canvas_pos, Some(active_windows), zoom) {
                match region {
                    WindowRegion::CloseButton => {
                        self.close_window(window_id);
                        return InputResult::Handled;
                    }
                    WindowRegion::MinimizeButton => {
                        self.minimize_window(window_id);
                        return InputResult::Handled;
                    }
                    WindowRegion::MaximizeButton => {
                        self.maximize_window(window_id);
                        return InputResult::Handled;
                    }
                    WindowRegion::TitleBar => {
                        self.camera_animation = None;
                        self.focus_window(window_id);
                        if let Some(window) = self.windows.get(window_id) {
                            self.input.start_window_move(window_id, canvas_pos - window.position);
                        }
                        return InputResult::Handled;
                    }
                    WindowRegion::Content => {
                        self.focus_window(window_id);
                        if let Some(window) = self.windows.get(window_id) {
                            // If content_interactive is false, clicking/dragging content moves the window
                            // If content_interactive is true, forward events to the app
                            if !window.content_interactive {
                                self.camera_animation = None;
                                self.input.start_window_move(window_id, canvas_pos - window.position);
                                return InputResult::Handled;
                            } else {
                                let local = canvas_pos - window.position;
                                return InputResult::Forward {
                                    window_id,
                                    local_x: local.x,
                                    local_y: local.y,
                                };
                            }
                        }
                    }
                    handle if handle.is_resize() => {
                        self.camera_animation = None;
                        self.focus_window(window_id);
                        if let Some(window) = self.windows.get(window_id) {
                            self.input.start_window_resize(window_id, handle, window.position, window.size, canvas_pos);
                        }
                        return InputResult::Handled;
                    }
                    _ => {}
                }
            }
        }

        InputResult::Unhandled
    }

    /// Handle pointer move
    pub fn handle_pointer_move(&mut self, x: f32, y: f32) -> InputResult {
        let screen_pos = Vec2::new(x, y);
        let canvas_pos = self.viewport.screen_to_canvas(screen_pos);

        if let Some(drag_state) = self.input.drag_state() {
            match drag_state {
                DragState::PanCanvas { start, start_center } => {
                    let delta = screen_pos - *start;
                    self.viewport.center = *start_center - delta / self.viewport.zoom;
                    return InputResult::Handled;
                }
                DragState::MoveWindow { window_id, offset } => {
                    let new_pos = canvas_pos - *offset;
                    let wid = *window_id;
                    self.move_window(wid, new_pos.x, new_pos.y);
                    return InputResult::Handled;
                }
                DragState::ResizeWindow { window_id, handle, start_pos, start_size, start_mouse } => {
                    let delta = canvas_pos - *start_mouse;
                    let (new_pos, new_size) = crate::input::calculate_resize(*handle, *start_pos, *start_size, delta);
                    let wid = *window_id;
                    self.move_window(wid, new_pos.x, new_pos.y);
                    self.resize_window(wid, new_size.width, new_size.height);
                    return InputResult::Handled;
                }
            }
        }

        InputResult::Unhandled
    }

    /// Handle pointer up
    pub fn handle_pointer_up(&mut self) -> InputResult {
        if self.input.is_dragging() {
            let was_pan = matches!(self.input.drag_state(), Some(DragState::PanCanvas { .. }));
            self.input.end_drag();

            if was_pan {
                self.commit_viewport_to_desktop();
            }

            return InputResult::Handled;
        }
        InputResult::Unhandled
    }

    /// Handle wheel event
    pub fn handle_wheel(&mut self, _dx: f32, dy: f32, x: f32, y: f32, ctrl: bool) -> InputResult {
        if ctrl {
            let factor = if dy < 0.0 { 1.1 } else { 0.9 };
            self.zoom_at(factor, x, y);
            InputResult::Handled
        } else {
            InputResult::Unhandled
        }
    }

    // =========================================================================
    // Void Transitions
    // =========================================================================

    /// Enter the void (zoom out to see all desktops)
    pub fn enter_void(&mut self, now_ms: f64) {
        if self.input.is_dragging() || !self.view_mode.is_desktop() || self.is_crossfading() {
            return;
        }

        let from_desktop = match self.view_mode {
            ViewMode::Desktop { index } => index,
            _ => return,
        };

        // Save current desktop camera state
        self.desktops.save_desktop_camera(from_desktop, self.viewport.center, self.viewport.zoom);

        // Calculate void camera to show all desktops
        let bounds: Vec<Rect> = self.desktops.desktops().iter().map(|d| d.bounds).collect();
        let center = VoidState::calculate_void_center(&bounds);
        let zoom = VoidState::calculate_fit_zoom(&bounds, self.viewport.screen_size);
        self.void_state.set_camera(Camera::at(center, zoom));

        // Start crossfade to void
        self.crossfade = Some(Crossfade::to_void(now_ms, from_desktop));
        self.last_activity_ms = now_ms;
    }

    /// Exit the void into a specific desktop
    pub fn exit_void(&mut self, desktop_index: usize, now_ms: f64) {
        if self.input.is_dragging() || !self.view_mode.is_void() || self.is_crossfading() {
            return;
        }

        // Switch to target desktop
        self.desktops.switch_to(desktop_index);

        // Start crossfade to desktop
        self.crossfade = Some(Crossfade::to_desktop(now_ms, desktop_index));
        self.last_activity_ms = now_ms;
    }

    // =========================================================================
    // Animation State
    // =========================================================================

    /// Check if any animation/activity is happening
    pub fn is_animating(&self, now_ms: f64) -> bool {
        if self.is_crossfading() || self.camera_animation.is_some() {
            return true;
        }
        // Check for recent manual pan/zoom activity (within 200ms)
        let activity_threshold_ms = 200.0;
        now_ms - self.last_activity_ms < activity_threshold_ms
    }

    /// Check if a viewport animation is in progress
    pub fn is_animating_viewport(&self) -> bool {
        self.is_crossfading()
    }

    /// Mark viewport activity (for is_animating check)
    pub fn mark_activity(&mut self, now_ms: f64) {
        self.last_activity_ms = now_ms;
    }

    /// Get the workspace index that should be rendered visually
    pub fn get_visual_active_workspace_at(&self, now_ms: f64) -> usize {
        if let Some(ref crossfade) = self.crossfade {
            match crossfade.direction {
                CrossfadeDirection::SwitchDesktop => {
                    // During desktop switch: show source in first half, target in second half
                    let progress = crossfade.progress(now_ms);
                    if progress < 0.5 {
                        // First half: show source desktop fading out
                        crossfade.source_desktop.unwrap_or_else(|| self.desktops.active_index())
                    } else {
                        // Second half: show target desktop fading in
                        crossfade.target_desktop.unwrap_or_else(|| self.desktops.active_index())
                    }
                }
                _ => {
                    // For other transitions, use target if available
                    if let Some(target) = crossfade.target_desktop {
                        return target;
                    }
                    self.desktops.active_index()
                }
            }
        } else {
            self.desktops.active_index()
        }
    }

    /// Get the workspace index that should be rendered visually (uses current time estimate)
    pub fn get_visual_active_workspace(&self) -> usize {
        // Use last_activity_ms as a rough timestamp estimate
        self.get_visual_active_workspace_at(self.last_activity_ms)
    }

    // =========================================================================
    // Window Screen Coordinates
    // =========================================================================

    /// Get window screen rects for rendering
    pub fn get_window_screen_rects(&self, now_ms: f64) -> Vec<WindowScreenRect> {
        let workspace_index = self.get_visual_active_workspace_at(now_ms);
        let workspace = match self.desktops.desktops().get(workspace_index) {
            Some(ws) => ws,
            None => return Vec::new(),
        };

        let focused_id = self.windows.focused();
        let opacity = self.calculate_window_opacity(now_ms);

        self.windows
            .windows_by_z()
            .into_iter()
            .filter(|w| workspace.contains_window(w.id) && w.state != WindowState::Minimized)
            .map(|w| {
                let screen_pos = self.viewport.canvas_to_screen(w.position);
                let screen_size = w.size.scale(self.viewport.zoom);

                WindowScreenRect {
                    id: w.id,
                    title: w.title.clone(),
                    app_id: w.app_id.clone(),
                    state: w.state,
                    focused: focused_id == Some(w.id),
                    screen_rect: Rect::new(
                        screen_pos.x,
                        screen_pos.y,
                        screen_size.width,
                        screen_size.height,
                    ),
                    opacity,
                    content_interactive: w.content_interactive,
                }
            })
            .collect()
    }

    /// Calculate window opacity based on transition state
    fn calculate_window_opacity(&self, now_ms: f64) -> f32 {
        match &self.crossfade {
            Some(crossfade) => {
                // Use the crossfade's computed opacity for smooth transitions
                let (desktop_opacity, _void_opacity) = crossfade.opacities(now_ms);
                desktop_opacity
            }
            None => 1.0,
        }
    }

    // =========================================================================
    // App Launch
    // =========================================================================

    /// Launch an application (creates window with app_id)
    pub fn launch_app(&mut self, app_id: &str) -> WindowId {
        // #region agent log
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" { fn fetch(url: &str, options: &JsValue) -> js_sys::Promise; }
        // #endregion
        
        let screen_w = self.viewport.screen_size.width;
        let screen_h = self.viewport.screen_size.height;

        let taskbar_height = 48.0;
        let padding = 20.0;
        let max_w = (screen_w - padding * 2.0).max(400.0);
        let max_h = (screen_h - taskbar_height - padding * 2.0).max(300.0);

        let win_w = 900.0_f32.min(max_w);
        let win_h = 600.0_f32.min(max_h);

        // Position will be calculated by create_window's cascade logic
        // With drag threshold detection in React, all windows can be draggable
        // contentInteractive is kept for future use if needed, but defaults to false
        let (title, content_interactive) = match app_id {
            "terminal" => ("Terminal", false),
            "browser" => ("Browser", false),
            "settings" => ("Settings", false),
            _ => (app_id, false),
        };

        let config = WindowConfig {
            title: title.to_string(),
            position: None,  // Let create_window handle cascading
            size: Size::new(win_w, win_h),
            min_size: Some(Size::new(200.0, 150.0)),
            max_size: None,
            app_id: app_id.to_string(),
            process_id: None,
            content_interactive,
        };

        self.create_window(config)
    }

    // =========================================================================
    // Camera Animation
    // =========================================================================

    /// Pan the camera to center on a window
    pub fn pan_to_window(&mut self, id: WindowId, now_ms: f64) {
        // #region agent log
        use wasm_bindgen::prelude::*;
        #[wasm_bindgen]
        extern "C" { fn fetch(url: &str, options: &JsValue) -> js_sys::Promise; }
        let log_before = format!("{{\"location\":\"engine.rs:938\",\"message\":\"pan_to_window - before\",\"data\":{{\"windowId\":{},\"viewportCenterX\":{},\"viewportCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"PAN\"}}", id, self.viewport.center.x, self.viewport.center.y, js_sys::Date::now() as u64);
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &"method".into(), &"POST".into()).ok();
        let headers = js_sys::Object::new();
        js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into()).ok();
        js_sys::Reflect::set(&opts, &"headers".into(), &headers).ok();
        js_sys::Reflect::set(&opts, &"body".into(), &log_before.into()).ok();
        let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts);
        // #endregion
        
        if let Some(window) = self.windows.get(id) {
            // Check if we have a saved camera position for this window
            let target_camera = if let Some(saved_camera) = self.window_cameras.get(&id) {
                // Restore the saved camera position for this window
                *saved_camera
            } else {
                // First time viewing this window - center on it
                let target_center = window.position + window.size.as_vec2() * 0.5;
                Camera::at(target_center, self.viewport.zoom)
            };

            self.camera_animation = Some(CameraAnimation::new(
                self.viewport.to_camera(),
                target_camera,
                now_ms,
            ));
            self.last_activity_ms = now_ms;
            
            // #region agent log
            let log_after = format!("{{\"location\":\"engine.rs:956\",\"message\":\"pan_to_window - after\",\"data\":{{\"windowId\":{},\"targetCenterX\":{},\"targetCenterY\":{}}},\"timestamp\":{},\"sessionId\":\"debug-session\",\"hypothesisId\":\"PAN\"}}", id, target_camera.center.x, target_camera.center.y, js_sys::Date::now() as u64);
            let opts2 = js_sys::Object::new();
            js_sys::Reflect::set(&opts2, &"method".into(), &"POST".into()).ok();
            let headers2 = js_sys::Object::new();
            js_sys::Reflect::set(&headers2, &"Content-Type".into(), &"application/json".into()).ok();
            js_sys::Reflect::set(&opts2, &"headers".into(), &headers2).ok();
            js_sys::Reflect::set(&opts2, &"body".into(), &log_after.into()).ok();
            let _ = fetch("http://127.0.0.1:7243/ingest/e9acd9f2-6d2c-4f15-b766-32e7c989e962", &opts2);
            // #endregion
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_engine_init() {
        let mut engine = DesktopEngine::new();
        engine.init(1920.0, 1080.0);

        assert!((engine.viewport.screen_size.width - 1920.0).abs() < 0.001);
        assert_eq!(engine.desktops.desktops().len(), 1);
    }

    #[test]
    fn test_desktop_engine_window_creation() {
        let mut engine = DesktopEngine::new();
        engine.init(1920.0, 1080.0);

        let id = engine.create_window(WindowConfig {
            title: "Test Window".to_string(),
            position: Some(Vec2::new(100.0, 100.0)),
            size: Size::new(800.0, 600.0),
            app_id: "test".to_string(),
            ..Default::default()
        });

        assert!(engine.windows.get(id).is_some());
        assert_eq!(engine.desktops.active_desktop().windows.len(), 1);
    }

    #[test]
    fn test_desktop_engine_create_desktop() {
        let mut engine = DesktopEngine::new();
        engine.init(1920.0, 1080.0);

        engine.create_desktop("Second");
        engine.create_desktop("Third");

        assert_eq!(engine.desktops.desktops().len(), 3);
    }

    #[test]
    fn test_viewport_pan() {
        let mut engine = DesktopEngine::new();
        engine.init(1920.0, 1080.0);

        engine.pan(-100.0, 0.0);

        assert!((engine.viewport.center.x - 100.0).abs() < 1.0);
    }
}
