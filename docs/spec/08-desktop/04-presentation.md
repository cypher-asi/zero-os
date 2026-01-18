# Presentation Layer

**Component:** 10-desktop/04-presentation  
**Status:** Specification

---

## Overview

The Presentation Layer uses React to render actual UI content inside windows.

---

## Integration Model

```
┌─────────────────────────────────────────────────────────────────┐
│  React Application                                               │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  <WindowContent windowId={123}>                              ││
│  │    <AppComponent />                                          ││
│  │  </WindowContent>                                            ││
│  └─────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│  Bridge Layer                                                    │
│  - Renders React to offscreen canvas                            │
│  - Provides input events to React                                │
│  - Sends texture updates to engine                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  WebGPU Engine                                                   │
│  - Composites window textures onto infinite canvas              │
└─────────────────────────────────────────────────────────────────┘
```

---

## Window Content Bridge

```typescript
// React component for window content
interface WindowContentProps {
  windowId: number;
  children: React.ReactNode;
}

function WindowContent({ windowId, children }: WindowContentProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  
  // Listen for window resize events
  useEffect(() => {
    const unsubscribe = windowManager.onResize(windowId, (newSize) => {
      setSize(newSize);
    });
    return unsubscribe;
  }, [windowId]);
  
  // Listen for input events forwarded from engine
  useEffect(() => {
    const unsubscribe = inputRouter.onWindowInput(windowId, (event) => {
      // Dispatch to React's event system
      dispatchSyntheticEvent(canvasRef.current, event);
    });
    return unsubscribe;
  }, [windowId]);
  
  // Render to offscreen canvas and send to engine
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    // Capture canvas content as texture
    const imageData = canvas.getContext('2d')?.getImageData(0, 0, size.width, size.height);
    if (imageData) {
      engine.updateWindowTexture(windowId, imageData);
    }
  });
  
  return (
    <canvas 
      ref={canvasRef} 
      width={size.width} 
      height={size.height}
      style={{ display: 'none' }}
    >
      {/* Render React content here */}
      {children}
    </canvas>
  );
}
```

---

## Component Model

```typescript
// Standard window component pattern
interface AppWindowProps {
  onClose?: () => void;
}

function SettingsWindow({ onClose }: AppWindowProps) {
  return (
    <div className="settings-panel">
      <h1>Settings</h1>
      <section>
        <h2>Appearance</h2>
        {/* Settings UI */}
      </section>
      <button onClick={onClose}>Close</button>
    </div>
  );
}

// Creating a window
function openSettings() {
  const windowId = windowManager.create({
    title: 'Settings',
    size: { width: 600, height: 400 },
    minSize: { width: 400, height: 300 },
  });
  
  ReactDOM.render(
    <WindowContent windowId={windowId}>
      <SettingsWindow onClose={() => windowManager.close(windowId)} />
    </WindowContent>,
    getWindowContainer(windowId)
  );
}
```

---

## State Management

Window UI state is managed locally in React, NOT in Axiom:

```typescript
// Local state for window-specific UI
function FileExplorer() {
  // These are ephemeral - not persisted to Axiom
  const [currentPath, setCurrentPath] = useState('/');
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  
  // File operations DO go through Axiom (via syscalls)
  const createFile = async (name: string) => {
    await syscall.createFile(currentPath, name);  // This is persisted
  };
  
  return (
    <div className="file-explorer">
      {/* UI rendered with local state */}
    </div>
  );
}
```

---

## Styling

```css
/* Standard window content styling */
.window-content {
  font-family: system-ui, -apple-system, sans-serif;
  background: var(--window-bg);
  color: var(--text-primary);
  padding: 16px;
}

/* Theme variables */
:root {
  --window-bg: #ffffff;
  --text-primary: #1a1a1a;
  --text-secondary: #666666;
  --accent: #0066cc;
  --border: #e0e0e0;
}

/* Dark theme */
[data-theme="dark"] {
  --window-bg: #1a1a1a;
  --text-primary: #ffffff;
  --text-secondary: #999999;
  --accent: #4d9fff;
  --border: #333333;
}
```

---

*[Back to Desktop](README.md) | [Previous: Input](03-input.md)*
