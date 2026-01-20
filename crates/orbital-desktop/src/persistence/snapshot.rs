//! Snapshot serialization for desktop state

use serde::{Deserialize, Serialize};
use crate::desktop::PersistedDesktop;

/// Snapshot of desktop state for persistence
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// Version for migration support
    pub version: u32,
    /// Active desktop index
    pub active_desktop: usize,
    /// Persisted desktop data
    pub desktops: Vec<PersistedDesktop>,
}

impl Snapshot {
    /// Current snapshot version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new snapshot
    pub fn new(active_desktop: usize, desktops: Vec<PersistedDesktop>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            active_desktop,
            desktops,
        }
    }

    /// Check if snapshot needs migration
    pub fn needs_migration(&self) -> bool {
        self.version < Self::CURRENT_VERSION
    }

    /// Migrate snapshot to current version
    pub fn migrate(&mut self) {
        // Add migration logic as versions increase
        self.version = Self::CURRENT_VERSION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Camera, Vec2};

    #[test]
    fn test_snapshot_creation() {
        let desktops = vec![
            PersistedDesktop {
                id: 1,
                name: "Main".to_string(),
                camera: Camera::new(),
            },
        ];
        let snapshot = Snapshot::new(0, desktops);
        
        assert_eq!(snapshot.version, Snapshot::CURRENT_VERSION);
        assert_eq!(snapshot.active_desktop, 0);
        assert_eq!(snapshot.desktops.len(), 1);
    }

    #[test]
    fn test_snapshot_serialization() {
        let desktops = vec![
            PersistedDesktop {
                id: 1,
                name: "Main".to_string(),
                camera: Camera::at(Vec2::new(100.0, 200.0), 1.5),
            },
        ];
        let snapshot = Snapshot::new(0, desktops);
        
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: Snapshot = serde_json::from_str(&json).unwrap();
        
        assert_eq!(restored.desktops[0].name, "Main");
        assert!((restored.desktops[0].camera.center.x - 100.0).abs() < 0.001);
    }
}
