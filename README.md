# Zero OS

A capability-based operating system with a WASM-first implementation approach.

## Project Status

**Phase 1: Kernel Bootstrap (WASM-First)** - Complete

This phase implements a minimal but spec-compliant kernel that runs in the browser via WebAssembly. The kernel implements process isolation (via Web Workers), capability-based access control, and IPC.

## Quick Start

### Prerequisites

- Rust (with `wasm32-unknown-unknown` target)
- wasm-pack

Install the WASM target if you haven't:

```bash
rustup target add wasm32-unknown-unknown
```

Install wasm-pack:

```bash
cargo install wasm-pack
```

### Building

Using Make (works on Windows with Make installed, macOS, Linux):

```bash
# Build and start the dev server
make dev

# Or step by step:
make build    # Build the WASM module
make server   # Start the dev server
```

Or manually:

```bash
cd crates/zos-supervisor-web && wasm-pack build --target web --out-dir www/pkg
cargo run -p dev-server
```

Then open http://localhost:8080 in your browser.

### Terminal Commands

Once the kernel boots, you can use these commands in the terminal:

#### Process Management
| Command | Description |
|---------|-------------|
| `ps` | List running processes |
| `spawn <type> [name]` | Spawn test process (types: memhog, sender, receiver, pingpong, idle) |
| `kill <pid>` | Kill a process |
| `inspect <pid>` | Show detailed process info |

#### Memory
| Command | Description |
|---------|-------------|
| `memstat` | Show memory usage per process |
| `alloc <pid> <bytes>` | Allocate memory to a process |
| `free <pid> <bytes>` | Free memory from a process |

#### IPC
| Command | Description |
|---------|-------------|
| `endpoints` / `ep` | List all IPC endpoints |
| `queue <endpoint_id>` | Show queue contents |
| `ipcstat` | IPC stats per process |
| `ipclog [count]` | Recent IPC traffic log |
| `send <pid> <message>` | Send message to a process |
| `caps [pid]` | List capabilities |

#### Testing
| Command | Description |
|---------|-------------|
| `burst <sender> <receiver> <count> <size>` | Send message burst |
| `ping <pid1> <pid2> [iterations]` | Ping-pong latency test |

#### System
| Command | Description |
|---------|-------------|
| `status` / `top` | System overview |
| `uptime` | Show system uptime |
| `datetime` | Show current date/time (UTC) |
| `echo <text>` | Echo text back |
| `clear` | Clear the screen |
| `exit` | Exit the terminal |

## Dashboard Features

The browser UI includes a real-time dashboard showing:

- **Process List**: All running processes with state, memory, and kill buttons
- **Memory Map**: Visual memory bar showing allocation per process
- **IPC Endpoints**: Endpoint list with queue depths and message counts
- **IPC Traffic**: Live feed of recent IPC messages

Quick-spawn buttons let you create test processes directly from the dashboard.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         BROWSER TAB                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    MAIN THREAD (Supervisor)                 │ │
│  │                                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐   │ │
│  │  │   Kernel    │  │ Capability  │  │    IPC Router    │   │ │
│  │  │   State     │  │   Tables    │  │                  │   │ │
│  │  └─────────────┘  └─────────────┘  └──────────────────┘   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                    postMessage (syscalls)                       │
│                              │                                   │
│  ┌──────────────────┐  ┌──────────────────┐                     │
│  │   Web Worker     │  │   Web Worker     │                     │
│  │   (init proc)    │  │   (terminal)     │                     │
│  │   PID: 1         │  │   PID: 2         │                     │
│  └──────────────────┘  └──────────────────┘                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Concepts

| OS Concept | WASM Implementation |
|------------|---------------------|
| **Process** | Web Worker (separate linear memory) |
| **Kernel/Supervisor** | Main thread (orchestrator) |
| **Syscall** | postMessage to supervisor |
| **IPC** | postMessage through supervisor |
| **Memory isolation** | WASM sandbox (implicit) |
| **Capability table** | Supervisor-managed Map |

## Project Structure

```
zero-os/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── zos-hal/                  # HAL trait (no_std)
│   ├── zos-kernel/               # Core kernel (no_std, includes mock HAL for testing)
│   ├── zos-process/              # Process-side syscall lib
│   ├── zos-apps/                 # Userspace apps (Terminal, Clock, Calculator, PermissionManager)
│   └── zos-system-procs/         # System processes
├── web/                          # Browser UI + processes
│   ├── desktop/                  # React desktop environment
│   ├── processes/                # Built WASM process binaries
│   └── pkg/                      # wasm-pack output (supervisor)
└── tools/
    └── dev-server/               # Static file server with COOP/COEP
```

## Development

```bash
make check    # Run cargo check
make test     # Run tests (18 unit tests)
make clean    # Clean build artifacts
make help     # Show all available commands
```

### Running Tests

```bash
cargo test --workspace
```

Tests include:
- Process registration and lifecycle
- Capability creation and granting
- Capability permission enforcement
- IPC send/receive
- IPC metrics tracking
- Memory allocation
- Syscall dispatch
- Traffic logging

### Building for Release

```bash
cd crates/zos-supervisor-web
wasm-pack build --target web --out-dir www/pkg --release
```

## Phase 1 Exit Criteria

- [x] `cargo build --target wasm32-unknown-unknown` succeeds
- [x] `cargo test` passes (18 tests)
- [x] Dev server runs (`cargo run -p dev-server`)
- [x] Browser shows terminal + dashboard at http://localhost:8080
- [x] `ps` shows: PID 1 (init), PID 2 (terminal)
- [x] `caps` shows terminal's capabilities
- [x] `spawn memhog` creates new process
- [x] `kill <pid>` terminates process
- [x] `memstat` shows real memory per process
- [x] `alloc <pid> <bytes>` allocates memory
- [x] Dashboard memory bar updates
- [x] `ipcstat` shows accurate message counts
- [x] `endpoints` shows queue depths
- [x] `ping` measures IPC latency
- [x] `burst` tests IPC throughput
- [x] Dashboard shows IPC traffic in real-time

## Documentation

- [Implementation Roadmap](docs/implementation/00-roadmap.md)
- [Phase 1: Hosted Simulator](docs/implementation/01-phase-hosted-simulator.md)
- [Kernel Specification](docs/specs/00-kernel/01-kernel.md)
- [Capability System](docs/specs/03-capability/01-capabilities.md)

## License

MIT
