# Persistence

## Overview

Desktop state can be serialized to snapshots for persistence across sessions. This enables restoring window positions, desktop layouts, and preferences.

## Snapshot Structure

```rust
pub struct Snapshot {
    /// Version for migration
    pub version: u32,
    
    /// Serialized desktops
    pub desktops: Vec<PersistedDesktop>,
    
    /// Serialized windows
    pub windows: Vec<PersistedWindow>,
    
    /// Active desktop index
    pub active_desktop: usize,
    
    /// Viewport state
    pub viewport: PersistedViewport,
}

pub struct PersistedDesktop {
    pub name: String,
    pub background: Option<String>,
    pub window_ids: Vec<u64>,
    pub camera: Option<PersistedCamera>,
}

pub struct PersistedWindow {
    pub id: u64,
    pub title: String,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub state: u8,
    pub app_id: String,
    pub process_id: Option<u64>,
}

pub struct PersistedViewport {
    pub center: (f32, f32),
    pub zoom: f32,
}

pub struct PersistedCamera {
    pub center: (f32, f32),
    pub zoom: f32,
}
```

## Creating Snapshots

```rust
impl DesktopEngine {
    pub fn create_snapshot(&self) -> Snapshot {
        Snapshot {
            version: 1,
            desktops: self.desktops.all()
                .map(|d| PersistedDesktop {
                    name: d.name.clone(),
                    background: d.background.clone(),
                    window_ids: d.window_ids.iter().copied().collect(),
                    camera: d.camera.map(|c| PersistedCamera {
                        center: (c.center.x, c.center.y),
                        zoom: c.zoom,
                    }),
                })
                .collect(),
            windows: self.windows.all_windows()
                .map(|w| PersistedWindow {
                    id: w.id,
                    title: w.title.clone(),
                    position: (w.position.x, w.position.y),
                    size: (w.size.width, w.size.height),
                    state: w.state as u8,
                    app_id: w.app_id.clone(),
                    process_id: w.process_id,
                })
                .collect(),
            active_desktop: self.desktops.active_index(),
            viewport: PersistedViewport {
                center: (self.viewport.center.x, self.viewport.center.y),
                zoom: self.viewport.zoom,
            },
        }
    }
}
```

## Restoring from Snapshots

```rust
impl DesktopEngine {
    pub fn restore_snapshot(&mut self, snapshot: Snapshot) {
        // Clear current state
        self.windows = WindowManager::new();
        self.desktops = DesktopManager::new();
        
        // Restore desktops
        for (i, pd) in snapshot.desktops.iter().enumerate() {
            if i == 0 {
                // Rename default desktop
                self.desktops.rename(0, &pd.name);
            } else {
                self.desktops.create(&pd.name);
            }
            
            if let Some(ref bg) = pd.background {
                self.set_desktop_background(i, bg);
            }
            
            if let Some(ref cam) = pd.camera {
                self.desktops.save_desktop_camera(
                    i,
                    Vec2::new(cam.center.0, cam.center.1),
                    cam.zoom,
                );
            }
        }
        
        // Restore windows
        for pw in snapshot.windows {
            let config = WindowConfig {
                title: pw.title,
                position: Some(Vec2::new(pw.position.0, pw.position.1)),
                size: Size::new(pw.size.0, pw.size.1),
                app_id: pw.app_id,
                process_id: pw.process_id,
                ..Default::default()
            };
            
            let id = self.windows.create(config);
            
            // Restore window state
            match pw.state {
                1 => self.windows.minimize(id),
                2 => self.windows.maximize(id, None),
                _ => {}
            }
            
            // Add to correct desktop
            // ...
        }
        
        // Restore viewport
        self.viewport.center = Vec2::new(
            snapshot.viewport.center.0,
            snapshot.viewport.center.1,
        );
        self.viewport.zoom = snapshot.viewport.zoom;
        
        // Switch to active desktop
        self.desktops.switch_to(snapshot.active_desktop);
    }
}
```

## Storage Integration

On WASM, snapshots can be stored in localStorage or IndexedDB:

```typescript
// Save snapshot
const snapshot = desktop.create_snapshot();
localStorage.setItem('desktop_snapshot', snapshot);

// Restore snapshot
const saved = localStorage.getItem('desktop_snapshot');
if (saved) {
    desktop.restore_snapshot(saved);
}
```

## Version Migration

Snapshots include a version number for forward compatibility:

```rust
impl Snapshot {
    pub fn migrate(&mut self, target_version: u32) {
        while self.version < target_version {
            match self.version {
                0 => {
                    // v0 -> v1: Add process_id field
                    for window in &mut self.windows {
                        window.process_id = None;
                    }
                    self.version = 1;
                }
                _ => break,
            }
        }
    }
}
```

## What's Not Persisted

Some state is not persisted:

- Running processes (must be respawned)
- Animation state
- Input/drag state
- Console buffers
- IPC message queues

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/persistence/*.rs`

### Key Invariants
- [ ] Snapshot version is incremented on breaking changes
- [ ] Migration handles all previous versions
- [ ] Window IDs are preserved for reference consistency
- [ ] Invalid snapshots don't crash (return error)

### Differences from v0.1.0
- Added process_id to PersistedWindow
- Added camera to PersistedDesktop
- Version migration system
- Background persisted per-desktop
