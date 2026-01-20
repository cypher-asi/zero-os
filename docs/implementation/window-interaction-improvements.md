# Window Interaction Improvements

## Overview

Windows can now be focused and dragged by clicking anywhere on the window, not just the title bar. Apps can opt-in to handling their own mouse events if they need interactivity (e.g., Terminal, Browser).

## Implementation

### 1. Window Configuration

Added a `content_interactive` field to control mouse event handling:

**Rust (`window/config.rs`, `window/window.rs`):**
```rust
pub struct WindowConfig {
    // ... existing fields ...
    /// Whether the window content area handles its own mouse events
    /// If false (default), clicking/dragging content will move the window
    /// If true, mouse events are forwarded to the app instead
    pub content_interactive: bool,
}

pub struct Window {
    // ... existing fields ...
    /// Whether the window content area handles its own mouse events
    pub content_interactive: bool,
}
```

### 2. Engine Input Handling

Modified `handle_pointer_down` in `engine.rs` to check the `content_interactive` flag:

```rust
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
```

### 3. Default Behavior

Updated `launch_app` in `engine.rs` to set appropriate defaults:

```rust
let (title, content_interactive) = match app_id {
    "terminal" => ("Terminal", true),  // Terminal needs to capture input
    "settings" => ("Settings", true),  // Settings may have interactive forms
    "browser" => ("Browser", true),    // Browser needs to capture clicks/input
    _ => (app_id, false),              // Default: draggable from anywhere
};
```

### 4. WASM Bindings

Added `content_interactive` parameter to the `create_window` API:

**Rust (`wasm.rs`):**
```rust
pub fn create_window(
    &mut self,
    title: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    app_id: &str,
    content_interactive: bool,
) -> u64
```

**TypeScript (`hooks/useSupervisor.ts`):**
```typescript
create_window(
  title: string,
  x: number,
  y: number,
  w: number,
  h: number,
  app_id: string,
  content_interactive: boolean
): bigint;
```

### 5. TypeScript Integration

**Updated `WindowInfo` interface:**
```typescript
export interface WindowInfo {
  id: number;
  title: string;
  appId: string;
  state: 'normal' | 'minimized' | 'maximized' | 'fullscreen';
  focused: boolean;
  zOrder: number;
  opacity: number;
  contentInteractive: boolean;  // NEW
  screenRect: { x: number; y: number; width: number; height: number };
}
```

**Updated `useWindowActions` hook:**
```typescript
const createWindow = useCallback(
  (
    title: string,
    x: number,
    y: number,
    w: number,
    h: number,
    appId: string,
    contentInteractive: boolean = false  // Default to draggable
  ) => {
    if (!desktop) return null;
    return Number(desktop.create_window(title, x, y, w, h, appId, contentInteractive));
  },
  [desktop]
);
```

### 6. React Component

Modified `WindowContent.tsx` to handle dragging from content area:

```typescript
<div 
  className={styles.content} 
  onPointerDown={(e) => {
    if (!win.focused) {
      focusWindow(win.id);
    }
    // If contentInteractive is false, allow dragging from content area
    // If contentInteractive is true, forward events to the app
    if (!win.contentInteractive) {
      desktopController?.start_window_drag(BigInt(win.id), e.clientX, e.clientY);
    }
    e.stopPropagation();
  }}
>
  {children}
</div>
```

## Usage

### Creating a Draggable Window

By default, windows are draggable from anywhere:

```typescript
const windowId = createWindow('My Window', 100, 100, 600, 400, 'myapp');
// contentInteractive defaults to false, so clicking anywhere moves the window
```

### Creating an Interactive Window

For apps that need to handle mouse events:

```typescript
const windowId = createWindow(
  'Terminal',
  100,
  100,
  800,
  600,
  'terminal',
  true  // contentInteractive = true, mouse events forwarded to app
);
```

## Behavior

### All Windows (Universal Drag Threshold)

**In the content area:**
- **Simple click**: Focuses window and forwards to app (buttons work, text selection works, etc.)
- **Click + drag** (> 5px movement): Starts dragging the window
- **Buttons/inputs**: Always work normally (drag detection skips these elements)

**Title bar:**
- **Click + drag**: Immediately starts dragging (no threshold)
- **Title bar buttons**: Work normally (minimize/maximize/close)

**Resize handles:**
- **Click + drag**: Resizes the window

This provides intuitive behavior for all window types:
- Terminal: Click to focus, type to input, click+drag to move
- Settings: Click buttons to interact, click+drag empty space to move
- Files: Click to select, click+drag to move
- Any app: Works the same way

## Apps Configuration

With the drag threshold approach, **all apps use the same configuration**:
- **Terminal**: `contentInteractive = false` - you can click to interact OR click+drag to move
- **Browser**: `contentInteractive = false` - you can click links OR click+drag to move
- **Settings**: `contentInteractive = false` - you can click buttons OR click+drag to move
- **Files**: `contentInteractive = false` - click+drag to move
- **All apps**: Default is `contentInteractive = false` with drag threshold

The `contentInteractive` flag is retained for potential future use but is currently set to `false` for all windows.

## Implementation Details

### Drag Threshold Detection

All windows now use a **drag threshold** approach to distinguish between clicks and drags:

1. **On pointer down**: 
   - Focus the window
   - Record the initial pointer position
   - Capture the pointer to track movement

2. **On pointer move**:
   - Calculate distance from initial position
   - If distance > 5 pixels → start window drag
   - If distance ≤ 5 pixels → wait for more movement or pointer up

3. **On pointer up**:
   - If drag wasn't started → treat as a normal click (app receives the event)
   - Release pointer capture

This approach provides the best of both worlds:
- **Simple click**: Goes to the application (you can click buttons, text inputs, etc.)
- **Click + drag**: Moves the window

The 5-pixel threshold prevents accidental drags from small hand movements while still feeling responsive.

## Benefits

1. **Universal Solution**: Same behavior for all windows - no special cases needed
2. **Smart Detection**: Distinguishes between clicks (for interaction) and drags (for movement)
3. **Better UX**: 
   - Click buttons/inputs → they work normally
   - Click+drag anywhere → moves the window
   - No need to "find" the title bar to move windows
4. **Intuitive**: Matches modern OS behavior (Windows 11, macOS, many Linux DEs)
5. **No Configuration**: All windows work the same way out of the box
6. **Maintains Functionality**: Title bar, resize handles, and buttons still work as expected
7. **Responsive**: 5px threshold feels instant while preventing accidental drags

## Testing

To test the drag threshold behavior:

### Terminal Window
1. **Simple click** in terminal → should focus window, cursor should appear
2. **Click + drag** (move mouse > 5px) → should move the window
3. **Click title bar + drag** → should immediately move window (no threshold)
4. **Click minimize button** → should minimize (no drag)
5. **Type text** → should work normally

### Settings Window
1. **Click a button** → button should activate (no drag)
2. **Click empty space** → should focus window
3. **Click empty space + drag** → should move window
4. **Click+drag on a button then move** → should move window (button click ignored if dragged)

### Files Window (or any app)
1. **Click in content** → should focus window
2. **Click + drag** anywhere → should move window
3. **Resize from corners/edges** → should resize
4. **Double-click title bar** → should maximize (if implemented)

### Edge Cases
- Click and hold (no movement) → no drag starts, just a long click
- Click and move 4px → no drag (under threshold)
- Click and move 6px → drag starts
- Click on input/button then drag → input/button ignored, window moves
