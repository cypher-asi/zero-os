# Stage 1.8: Desktop Environment

> **Goal**: Full desktop environment with infinite canvas, window management, and React-based applications.

## Overview

Implement the desktop environment as specified in [docs/spec/08-desktop/](../../spec/08-desktop/README.md). The desktop uses a two-layer architecture:

- **Engine Layer (WebGPU)**: Infinite canvas, window compositing, workspaces, visual effects
- **Presentation Layer (React)**: Actual UI content inside windows, menus, dialogs

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

**Key Design Decision**: Window state (positions, z-order, focus) is ephemeral and NOT logged to Axiom. Only process creation and file operations go through Axiom.

## Key Tasks

### 1. Engine Layer (WebGPU)

1. Initialize WebGPU device and surface (`apps/orbital-web/src/desktop/engine.ts`)
2. Implement `Viewport` with screen-to-canvas coordinate transforms
3. Implement `Scene` with background and workspace containers
4. Implement `WindowFrame` compositing with shadows and rounded corners
5. Add pan/zoom gestures with smooth animation
6. Implement workspace switching with animated transitions

### 2. Window Management

1. Create `WindowManager` class (`apps/orbital-web/src/desktop/windows.ts`)
2. Implement window lifecycle: create, close, focus, minimize, maximize
3. Implement window positioning: move, resize with constraints
4. Implement z-order and focus stack management
5. Add hit testing for window regions (title bar, content, resize handles)
6. Implement window state machine (Normal, Minimized, Maximized, Fullscreen)

### 3. Input Routing

1. Create `InputRouter` class (`apps/orbital-web/src/desktop/input.ts`)
2. Handle pointer events: click, drag, hover
3. Route title bar drags to window move
4. Route resize handle drags to window resize
5. Forward content area events to React windows
6. Handle keyboard events with focus routing
7. Implement canvas gestures: ctrl+scroll zoom, middle-click pan, pinch-zoom

### 4. Presentation Layer (React)

1. Create `WindowContent` bridge component (`apps/orbital-web/src/desktop/WindowContent.tsx`)
2. Implement offscreen canvas rendering for window content
3. Bridge input events from engine to React synthetic events
4. Create theme system with CSS variables (light/dark)
5. Build standard window components (title bar, close/minimize/maximize buttons)

### 5. Integration with Stage 1.7

1. Connect existing terminal process to a desktop window
2. Create app launcher/dock component
3. Implement process-to-window association
4. Add desktop context menu (right-click on canvas)
5. System tray / status bar
6. Optionally embed Stage 1.7 dashboard as a "Developer Tools" window

**Reuse from Stage 1.7:**
- All supervisor APIs (`get_process_list_json()`, `send_input()`, etc.)
- IndexedDB Axiom persistence (`AxiomStorage`)
- Terminal process and input handling
- Process spawn/kill functionality

**New APIs needed:**
- `supervisor.create_window(process_id)` - Associate a window with a process
- `supervisor.get_process_windows(process_id)` - Get windows for a process
- Window-aware input routing (keyboard to focused window's process)

## File Structure

```
apps/orbital-web/
  src/
    desktop/
      engine.ts           # WebGPU engine, viewport, scene
      windows.ts          # WindowManager, Window state
      input.ts            # InputRouter, event handling
      types.ts            # Shared types (Vec2, Size, Rect)
      shaders/
        window.wgsl       # Window frame shader
        blur.wgsl         # Gaussian blur effect
        shadow.wgsl       # Drop shadow shader
    components/
      Desktop.tsx         # Main desktop component
      WindowContent.tsx   # Bridge component for window content
      WindowFrame.tsx     # Window chrome (title bar, buttons)
      AppLauncher.tsx     # Application launcher
      Dock.tsx            # Taskbar/dock
    styles/
      desktop.css         # Desktop styles
      theme.css           # Theme variables (light/dark)
```

## API Surface

### WindowManager

```typescript
interface WindowManager {
  create(config: WindowConfig): WindowId;
  close(id: WindowId): void;
  focus(id: WindowId): void;
  move(id: WindowId, position: Vec2): void;
  resize(id: WindowId, size: Size): void;
  setState(id: WindowId, state: WindowState): void;
  getWindows(): Window[];
  getFocused(): WindowId | null;
}

interface WindowConfig {
  title: string;
  position?: Vec2;
  size: Size;
  minSize?: Size;
  maxSize?: Size;
  process: ProcessId;
}

type WindowState = 'normal' | 'minimized' | 'maximized' | 'fullscreen';
```

### DesktopEngine

```typescript
interface DesktopEngine {
  pan(delta: Vec2): void;
  zoom(factor: number, anchor: Vec2): void;
  gotoWorkspace(index: number): void;
  render(): void;
  updateWindowTexture(id: WindowId, imageData: ImageData): void;
}

interface Viewport {
  center: Vec2;
  zoom: number;
  screenSize: Size;
  screenToCanvas(screen: Vec2): Vec2;
  canvasToScreen(canvas: Vec2): Vec2;
}
```

### InputRouter

```typescript
interface InputRouter {
  handle(event: InputEvent): InputResult;
  onWindowInput(id: WindowId, callback: (event: InputEvent) => void): () => void;
}

type InputResult = 
  | { type: 'handled' }
  | { type: 'unhandled' }
  | { type: 'forward', windowId: WindowId, event: InputEvent };
```

## Test Criteria

- [ ] WebGPU initializes and renders to canvas
- [ ] Viewport pan/zoom works with mouse and touch
- [ ] Windows can be created and display content
- [ ] Window move via title bar drag
- [ ] Window resize via corner/edge drag
- [ ] Window focus changes z-order
- [ ] Close button closes window
- [ ] Minimize/maximize buttons work
- [ ] Keyboard input routes to focused window
- [ ] Multiple workspaces with navigation
- [ ] Terminal process runs in a window
- [ ] Theme switching (light/dark) works
- [ ] Visual effects (shadows, rounded corners) render correctly

## Dependencies

### New NPM Dependencies

```json
{
  "@webgpu/types": "^0.1.0"
}
```

### Browser Requirements

- WebGPU support (Chrome 113+, Firefox 118+, Safari 17+)
- Fallback to Canvas 2D for unsupported browsers

## What Does NOT Go Through Axiom

Per the spec, these are ephemeral and local-only:

- Window positions
- Z-order / stacking
- Focus state
- Open windows list
- Window sizes
- Desktop arrangement

## What DOES Go Through Axiom

- Application launches (process creation via syscall)
- File operations triggered by apps
- System configuration changes

## Next Stage

After Stage 1.8, Phase 1 is complete. Proceed to [Phase 2: QEMU](../phase-2-qemu/README.md) for hardware abstraction.
