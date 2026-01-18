//! Workspace Manager for the desktop environment
//!
//! Manages workspaces as regions on the infinite canvas.
//! Each workspace has a bounding rectangle and contains windows.
//!
//! ## Layout
//!
//! Workspaces are arranged in a grid on the infinite canvas.
//! The grid grows horizontally as workspaces are added.

use super::types::{Rect, Size, Vec2};
use super::windows::WindowId;

/// Unique workspace identifier
pub type WorkspaceId = u32;

/// A workspace region on the infinite canvas
#[derive(Clone, Debug)]
pub struct Workspace {
    /// Unique identifier
    pub id: WorkspaceId,
    /// Human-readable name
    pub name: String,
    /// Bounds on infinite canvas
    pub bounds: Rect,
    /// Windows in this workspace
    pub windows: Vec<WindowId>,
}

impl Workspace {
    /// Create a new workspace at the given bounds
    pub fn new(id: WorkspaceId, name: String, bounds: Rect) -> Self {
        Self {
            id,
            name,
            bounds,
            windows: Vec::new(),
        }
    }

    /// Add a window to this workspace
    pub fn add_window(&mut self, window_id: WindowId) {
        if !self.windows.contains(&window_id) {
            self.windows.push(window_id);
        }
    }

    /// Remove a window from this workspace
    pub fn remove_window(&mut self, window_id: WindowId) {
        self.windows.retain(|&id| id != window_id);
    }

    /// Check if workspace contains a window
    pub fn contains_window(&self, window_id: WindowId) -> bool {
        self.windows.contains(&window_id)
    }
}

/// Workspace manager for infinite canvas regions
pub struct WorkspaceManager {
    /// All workspaces
    workspaces: Vec<Workspace>,
    /// Currently active workspace index
    active: usize,
    /// Next workspace ID
    next_id: WorkspaceId,
    /// Standard workspace size
    workspace_size: Size,
    /// Gap between workspaces
    workspace_gap: f32,
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new() -> Self {
        Self {
            workspaces: Vec::new(),
            active: 0,
            next_id: 1,
            workspace_size: Size::new(1920.0, 1080.0),
            workspace_gap: 100.0,
        }
    }

    /// Create a new workspace
    pub fn create(&mut self, name: &str) -> WorkspaceId {
        let id = self.next_id;
        self.next_id += 1;

        // Calculate bounds - workspaces are arranged horizontally, centered on y-axis
        let index = self.workspaces.len();
        let x = index as f32 * (self.workspace_size.width + self.workspace_gap);
        // Center the bounds so that (0,0) is at the center of the first workspace
        let half_w = self.workspace_size.width / 2.0;
        let half_h = self.workspace_size.height / 2.0;
        let bounds = Rect::new(x - half_w, -half_h, self.workspace_size.width, self.workspace_size.height);

        let workspace = Workspace::new(id, name.to_string(), bounds);
        self.workspaces.push(workspace);

        // If this is the first workspace, set it as active
        if self.workspaces.len() == 1 {
            self.active = 0;
        }

        id
    }

    /// Switch to workspace by index
    /// Returns true if switched, false if index out of bounds
    pub fn switch_to(&mut self, index: usize) -> bool {
        if index < self.workspaces.len() {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// Get the center position of a workspace
    pub fn get_workspace_center(&self, index: usize) -> Option<Vec2> {
        self.workspaces.get(index).map(|ws| ws.bounds.center())
    }

    /// Get the currently active workspace
    pub fn active_workspace(&self) -> &Workspace {
        &self.workspaces[self.active]
    }

    /// Get the currently active workspace mutably
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active]
    }

    /// Get the active workspace index
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Get all workspaces
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    /// Get a workspace by ID
    pub fn get(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|ws| ws.id == id)
    }

    /// Get a workspace by ID mutably
    pub fn get_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|ws| ws.id == id)
    }

    /// Get workspace index by ID
    pub fn index_of(&self, id: WorkspaceId) -> Option<usize> {
        self.workspaces.iter().position(|ws| ws.id == id)
    }

    /// Add a window to a workspace
    pub fn add_window_to_workspace(&mut self, workspace_index: usize, window_id: WindowId) {
        if let Some(workspace) = self.workspaces.get_mut(workspace_index) {
            workspace.add_window(window_id);
        }
    }

    /// Remove a window from all workspaces
    pub fn remove_window(&mut self, window_id: WindowId) {
        for workspace in &mut self.workspaces {
            workspace.remove_window(window_id);
        }
    }

    /// Find which workspace contains a window
    pub fn workspace_containing(&self, window_id: WindowId) -> Option<&Workspace> {
        self.workspaces.iter().find(|ws| ws.contains_window(window_id))
    }

    /// Set workspace size (affects new workspaces)
    pub fn set_workspace_size(&mut self, size: Size) {
        self.workspace_size = size;
    }

    /// Get the number of workspaces
    pub fn count(&self) -> usize {
        self.workspaces.len()
    }

    /// Delete a workspace by index (cannot delete if it's the last one)
    pub fn delete(&mut self, index: usize) -> bool {
        if self.workspaces.len() <= 1 || index >= self.workspaces.len() {
            return false;
        }

        self.workspaces.remove(index);

        // Adjust active index if needed
        if self.active >= self.workspaces.len() {
            self.active = self.workspaces.len() - 1;
        }

        true
    }

    /// Rename a workspace
    pub fn rename(&mut self, index: usize, name: &str) {
        if let Some(workspace) = self.workspaces.get_mut(index) {
            workspace.name = name.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let mut wm = WorkspaceManager::new();

        let id1 = wm.create("Workspace 1");
        let id2 = wm.create("Workspace 2");

        assert_eq!(wm.count(), 2);
        assert!(wm.get(id1).is_some());
        assert!(wm.get(id2).is_some());
    }

    #[test]
    fn test_workspace_positions() {
        let mut wm = WorkspaceManager::new();

        wm.create("Workspace 1");
        wm.create("Workspace 2");

        let ws1 = &wm.workspaces()[0];
        let ws2 = &wm.workspaces()[1];

        // First workspace centered at origin
        let center1 = ws1.bounds.center();
        assert!((center1.x - 0.0).abs() < 0.001);
        assert!((center1.y - 0.0).abs() < 0.001);

        // Second workspace offset by workspace_size + gap
        let center2 = ws2.bounds.center();
        let expected_x = wm.workspace_size.width + wm.workspace_gap;
        assert!((center2.x - expected_x).abs() < 0.001);
    }

    #[test]
    fn test_workspace_switching() {
        let mut wm = WorkspaceManager::new();

        wm.create("Workspace 1");
        wm.create("Workspace 2");
        wm.create("Workspace 3");

        assert_eq!(wm.active_index(), 0);

        assert!(wm.switch_to(2));
        assert_eq!(wm.active_index(), 2);

        assert!(!wm.switch_to(10)); // Out of bounds
        assert_eq!(wm.active_index(), 2); // Should not change
    }

    #[test]
    fn test_workspace_windows() {
        let mut wm = WorkspaceManager::new();

        wm.create("Workspace 1");
        wm.create("Workspace 2");

        wm.add_window_to_workspace(0, 100);
        wm.add_window_to_workspace(0, 101);
        wm.add_window_to_workspace(1, 200);

        assert_eq!(wm.workspaces()[0].windows.len(), 2);
        assert_eq!(wm.workspaces()[1].windows.len(), 1);

        // Find workspace containing window
        let ws = wm.workspace_containing(100).unwrap();
        assert_eq!(ws.name, "Workspace 1");

        // Remove window from all workspaces
        wm.remove_window(100);
        assert_eq!(wm.workspaces()[0].windows.len(), 1);
    }
}
