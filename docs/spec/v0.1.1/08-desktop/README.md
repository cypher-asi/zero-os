# 08 - Desktop Compositor

## Overview

The `zos-desktop` crate provides the desktop environment functionality:

- Window management (create, close, focus, z-order)
- Desktop management (multiple infinite canvases)
- Viewport/camera transformations
- Input routing and hit testing
- Animated transitions between desktops

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                         DesktopEngine                                │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────────┐ │
│  │ WindowManager  │  │ DesktopManager │  │      InputRouter       │ │
│  │ • Windows map  │  │ • Desktops[]   │  │ • Drag state           │ │
│  │ • Z-order      │  │ • Active index │  │ • Hit testing          │ │
│  │ • Focus        │  │ • Cameras      │  │                        │ │
│  └────────────────┘  └────────────────┘  └────────────────────────┘ │
│                                                                      │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────────┐ │
│  │    Viewport    │  │   Transitions  │  │     Persistence        │ │
│  │ • Screen size  │  │ • Crossfade    │  │ • Snapshot             │ │
│  │ • Camera       │  │ • Camera anim  │  │ • Restore              │ │
│  │ • Zoom         │  │                │  │                        │ │
│  └────────────────┘  └────────────────┘  └────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

## Design Principles

1. **Pure Rust Core**: All state management is pure Rust, testable without browser
2. **Time Abstraction**: Animations use injectable time sources for deterministic testing
3. **Small Modules**: Each file stays under 300 lines for maintainability
4. **Minimal Dependencies**: Core types have no browser dependencies

## Module Structure

| Module | Purpose |
|--------|---------|
| `math` | Core geometry types (Vec2, Rect, Size, Camera) |
| `window` | Window lifecycle and management |
| `desktop` | Desktop (workspace) management |
| `input` | Input routing and drag state machine |
| `transition` | Animation and transition systems |
| `persistence` | State serialization for storage |

## Specification Sections

| Section | Description |
|---------|-------------|
| [01-engine.md](./01-engine.md) | DesktopEngine core |
| [02-windows.md](./02-windows.md) | Window management |
| [03-input.md](./03-input.md) | Input routing with drag threshold |
| [04-viewport.md](./04-viewport.md) | Viewport and camera system |
| [05-persistence.md](./05-persistence.md) | State snapshots |

## Example Usage

```rust
use zos_desktop::{DesktopEngine, WindowConfig, Size, Vec2};

let mut engine = DesktopEngine::new();
engine.init(1920.0, 1080.0);

// Create a window
let window_id = engine.create_window(WindowConfig {
    title: "My Window".to_string(),
    position: Some(Vec2::new(100.0, 100.0)),
    size: Size::new(800.0, 600.0),
    app_id: "my-app".to_string(),
    ..Default::default()
});

// Handle input
let result = engine.on_mouse_down(500.0, 400.0, 0.0);
```

## Separation from Supervisor

The desktop is separate from the supervisor:

| Concern | Desktop (zos-desktop) | Supervisor (zos-supervisor) |
|---------|----------------------|--------------------------------|
| Window management | Yes | No |
| Input routing | Yes | No |
| Process lifecycle | No | Yes |
| Syscall dispatch | No | Yes |
| Console callbacks | No | Yes |
| IPC routing | No | Yes |

This separation enables:
- Independent testing of desktop logic
- Clear boundary between OS primitives and UI
- Potential for alternative UIs (headless, CLI, etc.)

## React Integration

The desktop exports a WASM package (`zos_desktop.js`) that React components use:

```typescript
import { DesktopController } from 'zos_desktop';

const desktop = new DesktopController();
desktop.init(window.innerWidth, window.innerHeight);

// Get render state for React
const renderState = JSON.parse(desktop.render_state());
```

## Compliance Checklist

### Source Files
- `crates/zos-desktop/src/lib.rs` - Module exports
- `crates/zos-desktop/src/engine/*.rs` - DesktopEngine
- `crates/zos-desktop/src/window/*.rs` - Window management
- `crates/zos-desktop/src/desktop/*.rs` - Desktop management
- `crates/zos-desktop/src/input/*.rs` - Input routing

### Key Invariants
- [ ] Window IDs are unique
- [ ] Focused window is in z-order
- [ ] Drag threshold is 5px
- [ ] Transitions complete before allowing new ones

### Differences from v0.1.0
- Separated from supervisor into own crate
- Universal drag threshold (5px) for all windows
- Process ID linking for window lifecycle
- Camera position saved per-window
