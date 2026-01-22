# 07 - Applications

## Overview

Zero OS applications are WASM binaries that implement the `ZeroApp` trait. This provides a consistent lifecycle and communication model across all apps.

## ZeroApp Trait

```rust
pub trait ZeroApp {
    /// Returns the static application manifest
    fn manifest() -> &'static AppManifest where Self: Sized;

    /// Called once when the app starts
    fn init(&mut self, ctx: &AppContext) -> Result<(), AppError>;

    /// Called repeatedly in the event loop
    fn update(&mut self, ctx: &AppContext) -> ControlFlow;

    /// Called when a message is received via IPC
    fn on_message(&mut self, ctx: &AppContext, msg: Message) -> Result<(), AppError>;

    /// Called before the app exits
    fn shutdown(&mut self, ctx: &AppContext);
}
```

## Lifecycle

```
_start()
    │
    ▼
┌────────────────┐
│  Create app    │
│  instance      │
└───────┬────────┘
        │
        ▼
┌────────────────┐
│   init(ctx)    │  ← Initialize state, create endpoints
└───────┬────────┘
        │
        ▼
┌─────────────────────────────────────┐
│            Event Loop                │
│  ┌────────────────────────────────┐ │
│  │        update(ctx)             │ │
│  │  • Periodic work               │ │
│  │  • Returns ControlFlow         │ │
│  └────────────────────────────────┘ │
│                 │                    │
│  ┌──────────────┼──────────────┐    │
│  │              │              │    │
│  ▼              ▼              ▼    │
│ Continue       Yield          Exit  │
│ (next tick)   (yield CPU)    (done) │
│  │              │                   │
│  └──────────────┘                   │
│                                     │
│  ┌────────────────────────────────┐ │
│  │    on_message(ctx, msg)        │ ← IPC messages
│  └────────────────────────────────┘ │
└─────────────────────────────────────┘
        │
        ▼
┌────────────────┐
│  shutdown(ctx) │
└───────┬────────┘
        │
        ▼
      exit()
```

## AppContext

Provides execution context to app methods:

```rust
pub struct AppContext {
    /// This process's ID
    pub pid: u32,

    /// Monotonic uptime in nanoseconds (via SYS_GET_TIME)
    pub uptime_ns: u64,

    /// Wall-clock time in milliseconds since Unix epoch (via SYS_GET_WALLCLOCK)
    pub wallclock_ms: u64,

    /// Capability slot for communicating with UI (if connected)
    pub ui_endpoint: Option<u32>,

    /// Capability slot for receiving input
    pub input_endpoint: Option<u32>,
}
```

## ControlFlow

Returned by `update()` to control execution:

```rust
pub enum ControlFlow {
    /// Continue to next update cycle
    Continue,

    /// Exit with the given code
    Exit(i32),

    /// Yield CPU, wait for next scheduling quantum
    Yield,
}
```

## Message

IPC message structure:

```rust
pub struct Message {
    /// Message tag (identifies message type)
    pub tag: u32,

    /// Sender's process ID
    pub from_pid: u32,

    /// Message payload data
    pub data: Vec<u8>,
}
```

## AppManifest

Static metadata about the application:

```rust
pub struct AppManifest {
    /// Unique application ID (reverse domain notation)
    pub id: &'static str,

    /// Human-readable name
    pub name: &'static str,

    /// Version string
    pub version: &'static str,

    /// Required capabilities
    pub permissions: &'static [Permission],
}
```

## App Protocol

See [02-protocol.md](./02-protocol.md) for the wire format.

### Message Tags

| Tag | Name | Direction | Purpose |
|-----|------|-----------|---------|
| 0x2000 | MSG_APP_STATE | App → UI | State update |
| 0x2001 | MSG_APP_INPUT | UI → App | User input event |
| 0x2002 | MSG_UI_READY | UI → App | UI surface ready |
| 0x2003 | MSG_APP_FOCUS | App → UI | Request focus |
| 0x2004 | MSG_APP_ERROR | App → UI | Error notification |

### Type Tags

State types:
- 0x01: Clock state
- 0x02: Calculator state
- 0x10: Terminal state

Input types:
- 0x10: Button press
- 0x11: Text input
- 0x12: Key press
- 0x13: Focus change

## Example App: Clock

```rust
use zos_apps::*;

static MANIFEST: AppManifest = AppManifest {
    id: "com.zero.clock",
    name: "Clock",
    version: "1.0.0",
    permissions: &[],
};

#[derive(Default)]
struct ClockApp {
    last_update: u64,
}

impl ZeroApp for ClockApp {
    fn manifest() -> &'static AppManifest {
        &MANIFEST
    }

    fn init(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    fn update(&mut self, ctx: &AppContext) -> ControlFlow {
        // Send state update every 100ms
        if ctx.uptime_ns - self.last_update > 100_000_000 {
            self.send_state(ctx);
            self.last_update = ctx.uptime_ns;
        }
        ControlFlow::Yield
    }

    fn on_message(&mut self, _ctx: &AppContext, _msg: Message) -> Result<(), AppError> {
        Ok(())
    }

    fn shutdown(&mut self, _ctx: &AppContext) {}
}
```

## AppRuntime

The `AppRuntime` helper runs the event loop:

```rust
pub struct AppRuntime<A: ZeroApp> {
    app: A,
    ctx: AppContext,
}

impl<A: ZeroApp> AppRuntime<A> {
    pub fn run(app: A) -> ! {
        let mut runtime = Self::new(app);
        runtime.init();
        
        loop {
            runtime.update_context();
            
            // Poll for messages
            if let Some(msg) = receive(runtime.ctx.input_endpoint.unwrap_or(0)) {
                let _ = runtime.app.on_message(&runtime.ctx, msg.into());
            }
            
            // Call update
            match runtime.app.update(&runtime.ctx) {
                ControlFlow::Continue => {},
                ControlFlow::Yield => yield_now(),
                ControlFlow::Exit(code) => {
                    runtime.app.shutdown(&runtime.ctx);
                    exit(code);
                }
            }
        }
    }
}
```

## Compliance Checklist

### Source Files
- `crates/zos-apps/src/app.rs` - ZeroApp trait
- `crates/zos-apps/src/manifest.rs` - AppManifest
- `crates/zos-apps/src/runtime.rs` - AppRuntime
- `crates/zos-apps/src/error.rs` - AppError

### Key Invariants
- [ ] ZeroApp trait is object-safe (except manifest)
- [ ] AppContext provides accurate time values
- [ ] ControlFlow::Exit triggers shutdown
- [ ] Messages are delivered in order

### Differences from v0.1.0
- Added wallclock_ms to AppContext
- Console I/O uses syscall (not IPC slot)
- UI endpoint is optional (for non-UI apps)
- ControlFlow::Yield for cooperative scheduling
