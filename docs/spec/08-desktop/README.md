# Desktop Environment

**Layer:** 10  
**Status:** Specification

---

## Overview

The Desktop Environment provides the visual interface including window management, workspaces, and an infinite canvas.

---

## Architecture: Engine vs Presentation

```
┌─────────────────────────────────────────────────────────────────┐
│                    PRESENTATION LAYER (React)                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  App UI     │  │  Settings   │  │  Terminal   │  ... (React) │
│  │  Components │  │  Panel      │  │  Emulator   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
├─────────────────────────────────────────────────────────────────┤
│                    ENGINE LAYER (WebGPU)                         │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  Infinite Canvas  │  Window Frames  │  Workspaces  │ Effects ││
│  │  (pan/zoom)       │  (chrome)       │  (switching) │ (blur)  ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Responsibility |
|-------|------------|----------------|
| **Engine** | WebGPU | Infinite canvas, window compositing, workspaces, visual effects, pan/zoom |
| **Presentation** | React | Actual UI content inside windows, menus, dialogs, app interfaces |

---

## Key Design Decision: Ephemeral Window State

The desktop uses **local state variables** for UI state - **NOT Axiom**:

| State Type | Storage | Rationale |
|------------|---------|-----------|
| Window positions | Local (in-memory) | Ephemeral, user preference |
| Z-order / stacking | Local (in-memory) | Changes constantly |
| Focus state | Local (in-memory) | Ephemeral |
| Open windows | Local (in-memory) | Session-based |
| Window size | Local (in-memory) | User preference |

**What goes through Axiom:**
- Application launches (process creation)
- File operations triggered by apps
- System configuration changes

**What does NOT go through Axiom:**
- Window move/resize
- Focus changes
- Minimize/maximize
- Desktop arrangement

---

## Component Files

| File | Description |
|------|-------------|
| [01-engine.md](01-engine.md) | WebGPU engine: infinite canvas, compositing |
| [02-windows.md](02-windows.md) | Window management |
| [03-input.md](03-input.md) | Input handling and routing |
| [04-presentation.md](04-presentation.md) | React integration |

---

*[Back to Spec Index](../README.md)*
