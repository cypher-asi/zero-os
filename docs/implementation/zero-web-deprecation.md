# zos-supervisor Deprecation Plan

**Status:** In Progress  
**Target:** Replace with `zos-desktop` crate  
**Progress:** See [wasm-desktop progress](../progress/wasm-desktop/progress.md)

---

## Overview

The `zos-supervisor` crate is being deprecated in favor of a new `zos-desktop` crate. The existing implementation will be retained as a backup while the new crate is developed.

---

## Rationale

The current `zos-supervisor` implementation has grown organically and has several architectural limitations:

1. **Tight coupling** between compositor, window management, and React integration
2. **Code organization** does not follow strict size limits (60-line functions, 500-line files)
3. **Limited testability** due to deep WebGPU/DOM dependencies throughout
4. **Missing features** for a proper desktop compositor (Void mode, infinite canvas precision)

The new `zos-desktop` crate addresses these with:

- Clean module boundaries with explicit trait abstractions
- Strict code quality constraints enforced by design
- Testable core logic (math, layout, transitions) without browser dependencies
- Full "Void" overview mode with animated transitions
- Infinite canvas support with f64 precision and origin rebasing

---

## Migration Strategy

### Phase 1: Backup and Parallel Development

1. **Rename** `crates/zos-supervisor/` to `crates/zos-supervisor-legacy/`
2. **Create** new `crates/zos-desktop/` with clean architecture
3. **Update** workspace `Cargo.toml` to include both crates
4. **Keep** web frontend using legacy crate during development

### Phase 2: Feature Parity

Implement core features in `zos-desktop`:

| Feature | zos-supervisor | zos-desktop |
|---------|-------------|-----------------|
| Desktop management | Partial | Full (create/remove/switch) |
| Void overview | No | Yes |
| Infinite canvas | Basic | Full (f64 precision, origin rebasing) |
| Window management | Yes | Yes (improved API) |
| React surfaces | Yes | Yes (cleaner bridge) |
| Transitions | Basic | Full (easing, configurable) |
| Persistence | No | Yes (snapshot/restore) |

### Phase 3: Integration

1. **Update** web frontend to use `zos-desktop`
2. **Migrate** existing window/desktop state
3. **Test** all existing functionality
4. **Remove** legacy imports

### Phase 4: Cleanup

1. **Archive** `zos-supervisor-legacy` (move to `archive/` or separate branch)
2. **Remove** from workspace
3. **Update** documentation

---

## File Mapping

### Legacy Structure (zos-supervisor)

```
crates/zos-supervisor/
├── src/
│   ├── lib.rs
│   ├── background.rs
│   └── desktop/
│       ├── mod.rs        (1959 lines - too large)
│       ├── desktops.rs   (871 lines - too large)
│       ├── windows.rs    (641 lines - too large)
│       ├── input.rs
│       ├── transition.rs
│       └── types.rs
```

### New Structure (zos-desktop)

```
crates/zos-desktop/
├── src/
│   ├── lib.rs                 (public API facade)
│   ├── compositor/
│   │   ├── mod.rs             (coordination)
│   │   └── state.rs           (compositor state)
│   ├── scene/
│   │   ├── desktop.rs         (desktop scene)
│   │   └── void.rs            (void overview scene)
│   ├── window/
│   │   ├── manager.rs         (window lifecycle)
│   │   ├── chrome.rs          (frame rendering)
│   │   └── layout.rs          (constraints, snap)
│   ├── input/
│   │   ├── router.rs          (hit testing, routing)
│   │   ├── events.rs          (event types)
│   │   └── gestures.rs        (touch gestures)
│   ├── render/
│   │   ├── mod.rs             (render graph)
│   │   ├── pipeline.rs        (WebGPU pipelines)
│   │   └── passes/
│   │       ├── background.rs
│   │       ├── window.rs
│   │       └── effects.rs
│   ├── ui/
│   │   ├── bridge.rs          (DOM mount management)
│   │   └── mount.rs           (mount lifecycle)
│   ├── transition/
│   │   ├── system.rs          (transition engine)
│   │   └── easing.rs          (easing functions)
│   ├── math/
│   │   ├── camera.rs          (viewport, projection)
│   │   ├── geom.rs            (rect, vec2, size)
│   │   └── precision.rs       (origin rebasing)
│   └── persistence/
│       └── snapshot.rs        (state serialization)
```

---

## Backward Compatibility

During migration:

- Legacy crate remains functional
- No breaking changes to web frontend initially
- New crate provides similar (but improved) public API
- Gradual migration of consumers

---

## Timeline

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Backup and parallel setup | **Complete** |
| 2 | Feature parity in zos-desktop | **Complete** |
| 3 | Web frontend migration | **Complete** |
| 4 | Legacy cleanup | **In Progress** |

---

## Phase 3 Completion (January 2026)

The web frontend has been successfully migrated to use `zos-desktop`:

**Changes Made:**

1. **Desktop Controller Migration**
   - Web frontend now uses `DesktopController` from `zos-desktop` (not `zos-supervisor`)
   - All window management, viewport, and transition logic handled by `zos-desktop`

2. **Module Cleanup**
   - Removed `desktop/` module from `zos-supervisor/src/`
   - Moved `background/` module to `zos-desktop/src/`
   - Updated `lib.rs` with deprecation notice
   - `zos-supervisor` now only provides:
     - `Supervisor` (process/IPC management)
     - Re-exports `background` module from `zos-desktop`

3. **API Updates**
   - Fixed `tick_frame()` data structure to match TypeScript expectations
   - Added `workspaceInfo` and `workspaceDimensions` fields
   - Added `backgrounds` array to workspace info

4. **File Structure**
   ```
   crates/zos-supervisor/
   ├── src/
   │   ├── lib.rs           (Supervisor + HAL)
   │   ├── axiom.rs         (Axiom IPC)
   │   └── worker.rs        (Worker process management)
   
   crates/zos-desktop/
   ├── src/
   │   ├── lib.rs           (Public API)
   │   ├── engine.rs        (DesktopEngine core)
   │   ├── wasm.rs          (WASM bindings)
   │   ├── background/      (WebGPU background renderer)
   │   ├── desktop/         (Desktop management)
   │   ├── window/          (Window management)
   │   ├── viewport/        (Camera/viewport)
   │   ├── input/           (Input handling)
   │   └── transition/      (Animations)
   ```

**Verification:**

- System boots successfully
- Workers spawn correctly
- No console errors
- Desktop rendering works
- Window management functional

---

## References

- [Desktop Specification](../spec/08-desktop/README.md)
- [zos-desktop crate](../../crates/zos-desktop/)
