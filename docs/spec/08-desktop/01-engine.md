# Desktop Engine

**Component:** 10-desktop/01-engine  
**Status:** Specification

---

## Overview

The Desktop Engine uses WebGPU to render an infinite canvas with windows and workspaces.

---

## Engine Structure

```rust
/// Desktop engine
pub struct DesktopEngine {
    /// WebGPU device
    device: wgpu::Device,
    
    /// Render queue
    queue: wgpu::Queue,
    
    /// Surface for rendering
    surface: wgpu::Surface,
    
    /// Current viewport
    viewport: Viewport,
    
    /// Scene graph
    scene: Scene,
    
    /// Render pipeline
    pipeline: RenderPipeline,
}

/// Viewport (camera into the infinite canvas)
#[derive(Clone, Debug)]
pub struct Viewport {
    /// Center position on canvas
    pub center: Vec2,
    
    /// Zoom level (1.0 = 100%)
    pub zoom: f32,
    
    /// Screen size in pixels
    pub screen_size: Size,
}

impl Viewport {
    /// Convert screen coordinates to canvas coordinates
    pub fn screen_to_canvas(&self, screen: Vec2) -> Vec2 {
        let offset = screen - self.screen_size.as_vec2() / 2.0;
        self.center + offset / self.zoom
    }
    
    /// Convert canvas coordinates to screen coordinates
    pub fn canvas_to_screen(&self, canvas: Vec2) -> Vec2 {
        let offset = canvas - self.center;
        offset * self.zoom + self.screen_size.as_vec2() / 2.0
    }
}
```

---

## Infinite Canvas

```rust
/// Infinite canvas scene
pub struct Scene {
    /// Background
    background: Background,
    
    /// Workspaces
    workspaces: Vec<Workspace>,
    
    /// Active workspace index
    active_workspace: usize,
}

/// Workspace (region of the canvas)
pub struct Workspace {
    /// Workspace bounds on canvas
    pub bounds: Rect,
    
    /// Windows in this workspace
    pub windows: Vec<WindowId>,
    
    /// Workspace name
    pub name: String,
}

impl Scene {
    /// Render the scene
    pub fn render(&self, engine: &DesktopEngine, viewport: &Viewport) {
        // Render background
        self.background.render(engine, viewport);
        
        // Render visible workspaces
        for workspace in &self.workspaces {
            if viewport.intersects(&workspace.bounds) {
                workspace.render(engine, viewport);
            }
        }
    }
}
```

---

## Compositing

```rust
/// Window frame for compositing
pub struct WindowFrame {
    /// Window ID
    pub window_id: WindowId,
    
    /// Position on canvas
    pub position: Vec2,
    
    /// Size
    pub size: Size,
    
    /// Content texture
    pub texture: wgpu::Texture,
    
    /// Frame decoration style
    pub style: FrameStyle,
}

/// Frame decoration style
#[derive(Clone, Debug)]
pub struct FrameStyle {
    pub title_bar_height: f32,
    pub border_radius: f32,
    pub shadow_radius: f32,
    pub shadow_color: Color,
}

impl WindowFrame {
    /// Render the window frame and content
    pub fn render(&self, engine: &DesktopEngine, viewport: &Viewport) {
        // Calculate screen position
        let screen_pos = viewport.canvas_to_screen(self.position);
        let screen_size = self.size * viewport.zoom;
        
        // Skip if off-screen
        if !viewport.is_visible(self.position, self.size) {
            return;
        }
        
        // Render shadow
        engine.render_shadow(screen_pos, screen_size, &self.style);
        
        // Render frame background
        engine.render_rounded_rect(screen_pos, screen_size, self.style.border_radius);
        
        // Render content texture
        engine.render_texture(&self.texture, screen_pos, screen_size);
        
        // Render title bar
        engine.render_title_bar(screen_pos, screen_size, &self.style);
    }
}
```

---

## Pan and Zoom

```rust
impl DesktopEngine {
    /// Handle pan gesture
    pub fn pan(&mut self, delta: Vec2) {
        self.viewport.center -= delta / self.viewport.zoom;
    }
    
    /// Handle zoom gesture
    pub fn zoom(&mut self, factor: f32, anchor: Vec2) {
        // Zoom centered on anchor point
        let canvas_anchor = self.viewport.screen_to_canvas(anchor);
        
        self.viewport.zoom *= factor;
        self.viewport.zoom = self.viewport.zoom.clamp(0.1, 10.0);
        
        // Adjust center to keep anchor in place
        let new_screen_anchor = self.viewport.canvas_to_screen(canvas_anchor);
        let correction = anchor - new_screen_anchor;
        self.viewport.center -= correction / self.viewport.zoom;
    }
    
    /// Animate to workspace
    pub fn goto_workspace(&mut self, index: usize) {
        if let Some(workspace) = self.scene.workspaces.get(index) {
            // Animate viewport to workspace center
            self.animate_to(workspace.bounds.center(), 1.0);
        }
    }
    
    fn animate_to(&mut self, target: Vec2, target_zoom: f32) {
        // Smooth animation using interpolation
        // (Actual implementation would use a tween)
    }
}
```

---

## Visual Effects

```rust
impl DesktopEngine {
    /// Render blur effect
    pub fn render_blur(&self, region: Rect, radius: f32) {
        // Gaussian blur shader pass
    }
    
    /// Render shadow
    pub fn render_shadow(&self, pos: Vec2, size: Size, style: &FrameStyle) {
        // Box shadow with blur
    }
    
    /// Render rounded rectangle
    pub fn render_rounded_rect(&self, pos: Vec2, size: Size, radius: f32) {
        // SDF-based rounded rectangle
    }
}
```

---

*[Back to Desktop](README.md) | [Next: Windows](02-windows.md)*
