# Zero OS - Executive Summary

**Version:** 2.0  
**Status:** Whitepaper  
**Classification:** Public

---

## What Is Zero OS?

Zero OS is a general-purpose operating system built from first principles with a singular objective: **make system behavior verifiable, auditable, and deterministic in meaning** - while preserving parallel execution, high performance, and native hardware support.

---

## The Problem

### System Opacity

Modern operating systems are **opaque by design**. When something goes wrong:

- Debugging requires forensic archaeology through scattered logs
- Security audits rely on sampling and heuristics, not proof
- Crash recovery is probabilistic - "probably consistent" is the best guarantee
- There is no authoritative record of what actually happened

The question *"What did the system do?"* has no definitive answer in any mainstream OS today.

### The Autonomous AI Imperative

Artificial intelligence systems are rapidly evolving from decision-support tools into **autonomous agents** capable of planning, acting, coordinating, and executing tasks with minimal or no human intervention. These systems increasingly operate across critical domains: financial markets, infrastructure management, cybersecurity, logistics, communications, and governance.

Unlike traditional software, autonomous AI systems:

- **Continuously observe and act** on the world
- **Operate over long time horizons** with persistent state
- **Adapt their behavior** based on feedback
- **Interact with other autonomous systems** at machine timescales

As autonomy increases, the locus of control shifts away from humans and toward software-defined decision loops. Errors, misalignment, or exploitation are no longer isolated incidents - they can **compound, propagate, and escalate** without immediate human oversight.

**Without verifiability, autonomous AI systems become opaque actors embedded in critical infrastructure - difficult to debug, impossible to audit, and unsafe to entrust with high-impact responsibilities.**

As autonomy increases, verifiability transitions from a desirable property to a **prerequisite for stability and trust**.

---

## The Zero Solution

Zero OS introduces a fundamentally different architecture based on seven non-negotiable invariants:

| Invariant | Guarantee |
|-----------|-----------|
| **Authoritative History** | Exactly one totally-ordered, hash-chained record of state transitions (the Axiom) |
| **Deterministic Authority** | Control-plane state is a pure function of the Axiom - always reproducible |
| **Parallel Execution** | Full SMP support; nondeterminism allowed in execution, never in authority |
| **No Unauthorized Effects** | No externally visible side effect without explicit authorization |
| **Crash Safety** | Pre-commit work is discardable; post-commit effects are idempotent and retryable |
| **Verifiable Computation** | Any authoritative result can be independently verified via replay |
| **Cryptographic Identity** | All principals have policy-controlled cryptographic identities |

**The Central Principle: All consequential state transitions flow through the Policy Engine before being recorded to the Axiom.**

---

## Architecture by Layer

Zero OS is organized into distinct layers, from most fundamental to least:

```
LAYER 8: APPLICATIONS
  Deterministic Jobs, Visual OS

LAYER 7: USER-FACING SERVICES
  Terminal Service, Update Manager

LAYER 6: EXECUTION INFRASTRUCTURE
  Three-Phase Action Model, Verification and Receipts

LAYER 5: NETWORK and DEVICE
  Driver Manager, Network Service

LAYER 4: STORAGE
  Block Storage, Filesystem, Content-Addressed Store

LAYER 3: PROCESS and CAPABILITY
  Capability Service, Process Manager

LAYER 2: CORE AUTHORITY
  Axiom Sequencer, Policy Engine, Key Derivation Service, Identity Service

LAYER 1: BOOTSTRAP
  Supervisor

LAYER 0: KERNEL
  Scheduler, Memory, Capabilities, IPC, Interrupts

HARDWARE ABSTRACTION LAYER (HAL)
  Platform-independent interface to execution substrate
        │
        ├── WASM (Browser/WASI)
        ├── QEMU (virtio devices)
        └── Bare Metal (x86_64 hardware)
```

### Hardware Abstraction Layer (HAL)

The HAL is the foundation that enables Zero OS to run on multiple platforms from a **single codebase**. It provides a zero-cost compile-time abstraction over the execution substrate — whether that's a web browser, a virtual machine, or bare metal hardware.

Key properties:
- **Zero runtime overhead** — all HAL calls are monomorphized and inlined at compile time
- **Full hardware access** — on bare metal, the HAL does not limit SMP, DMA, or hardware features
- **Portable core** — Layers 0-8 compile once and run on any HAL implementation

### Layer 0: Kernel

The minimal microkernel, built on the HAL, provides exactly five services:
- Preemptive multitasking (SMP)
- Virtual memory and address-space isolation
- Capability enforcement
- Fast IPC primitives
- Interrupt and timer handling

**Everything else runs in user space.**

### Layer 1: Bootstrap

The first user-space process (PID 1) has two phases:
- **Init** - Boot-time phase that starts all services in dependency order
- **Supervisor** - Runtime phase that monitors service health and restarts failed services

Init receives full system capabilities from the kernel and distributes them to services as needed.

### Layer 2: Core Authority (The "Authority Spine")

Four critical services form the foundation of system trust:

| Service | Role |
|---------|------|
| **Axiom Sequencer** | Single source of truth - append-only, hash-chained log of all state transitions |
| **Policy Engine** | Central authorization gate - ALL proposals must pass policy before reaching Axiom |
| **Key Derivation Service** | Cryptographic foundation - key derivation, signing within secure boundary |
| **Identity Service** | Principal management - users, services, nodes with hierarchical crypto keys |

### Layer 3: Process and Capability

- **Capability Service** - Manages capability delegation and revocation
- **Process Manager** - Creates/destroys processes, enforces resource limits

### Layer 4: Storage

- **Block Storage** - Low-level block device abstraction
- **Filesystem Service** - Namespace, metadata, path resolution
- **Content-Addressed Store** - Immutable content by BLAKE3 hash

### Layer 5: Network and Device

- **Driver Manager** - Loads and manages user-space drivers
- **Network Service** - TCP/IP stack, policy-gated connections

### Layer 6: Execution Infrastructure

- **Three-Phase Action Model** - Proposal, Policy, Commit, Effect lifecycle
- **Verification and Receipts** - Deterministic verification, cryptographic receipts

### Layer 7: User-Facing Services

- **Terminal Service** - User interaction, command execution
- **Update Manager** - Atomic system image updates, rollback

### Layer 8: Applications

- **Deterministic Jobs** - Content-addressed inputs to content-addressed outputs
- **Visual OS** - Deterministic UI layer (future)

---

## Key Differentiators

### vs. Verified Microkernels (seL4)
seL4 proves the kernel is correct. Zero proves the **system behaved correctly**. The Axiom provides what seL4 lacks: an authoritative history of every meaningful state transition.

### vs. Unix/Linux
Linux logs are advisory artifacts that may be incomplete, reordered, or lost. The Zero Axiom is the **source of truth** - if it is not in the Axiom, it did not happen.

### vs. Urbit
Urbit sacrifices parallelism for determinism (single-threaded execution). Zero achieves **both**: parallel execution with deterministic authority through the three-phase action model.

---

## Three-Phase Action Model

Every meaningful system action follows this lifecycle, with **mandatory Policy Engine evaluation**:

```
PHASE 1         POLICY          PHASE 2         PHASE 3
Pre-Commit  ->  ENGINE      ->  Commit      ->  Effect
(Propose)       (Authorize)     (Decide)        (Materialize)
    |               |               |               |
Tentative       Identity +      Axiom entry     Idempotent
execution       policy check    accepted        side effects
```

**Key guarantee: The Axiom only accepts entries that have been authorized by the Policy Engine.**

**Crash guarantees:**
- Crash before commit: proposal discarded (no effect)
- Crash after commit: effects retried (idempotent)

---

## Target Platforms

Zero OS compiles to multiple platforms from a single codebase via the Hardware Abstraction Layer (HAL):

| Platform | HAL Implementation | Use Case |
|----------|-------------------|----------|
| **WASM** | Browser (web-sys) or WASI runtime | Development, testing, web deployment, sandboxed AI agents |
| **QEMU** | Virtualized environment with virtio devices | Integration testing, production VMs, cloud deployment |
| **Bare Metal** | Native x86_64 hardware | Production deployment, full hardware access, maximum performance |

All platforms run the **same Zero OS core** — the Axiom, Policy Engine, and all services are identical. Only the HAL implementation differs.

### Platform Capabilities

| Capability | WASM | QEMU | Bare Metal |
|------------|------|------|------------|
| Threading | Web Workers | Full SMP | Full SMP |
| Preemption | Cooperative | Timer interrupt | Timer interrupt |
| Storage | IndexedDB/OPFS | virtio-blk | AHCI/NVMe |
| Network | WebSocket/Fetch | virtio-net | Real NIC |
| Performance | Good | Near-native | Native |

---

## Application Model (v0)

Version 0 supports **deterministic applications only**:

- Execute as discrete jobs with explicit inputs/outputs
- Content-addressed inputs and outputs
- Pinned execution environment
- No hidden nondeterminism (wall-clock, implicit randomness, etc.)
- Results are replay-verifiable

Interactive, long-running, nondeterministic applications are reserved for future versions.

---

## Why Now?

The convergence of several factors makes Zero OS not just timely, but necessary:

1. **Rust maturity** - Systems language with memory safety, no GC
2. **Hardware capability** - Modern CPUs can afford the three-phase overhead
3. **Security demands** - Supply-chain attacks require verifiable builds
4. **Trust verification** - Users demand transparency; "trust me" is no longer acceptable
5. **The rise of autonomous AI** - Autonomous agents require verifiable substrates to operate safely at scale
6. **AGI preparedness** - Systems approaching artificial general intelligence will require broad access to tools, persistent memory, self-modification capability, and cross-domain authority - all of which demand structural security guarantees

**As AI systems become increasingly autonomous, security and safety converge with systems engineering.** The question is no longer how to secure AI systems after they are built, but how to build systems that remain safe, stable, and governable as intelligence and autonomy increase.

Autonomous AI must run on **verifiable substrates** - operating systems designed from first principles for determinism, isolation, auditability, and trust. Without such foundations, the deployment of autonomous intelligence at scale represents an unacceptable systemic risk.

---

## What This Whitepaper Contains

This document suite provides:

- **Background and Motivation** - Why existing approaches fall short
- **Core Principles** - The seven invariants in detail
- **Architecture by Layer** - System structure from kernel to applications
- **Comparative Analysis** - Technical comparison with seL4, Linux, Plan 9, Urbit

The accompanying **[Specifications](../spec/README.md)** provide formal definitions organized by layer:

```
spec/
  00-hal/             # Hardware Abstraction Layer
  01-kernel/          # Scheduler, Memory, IPC, Syscalls, Interrupts
  02-boot/            # Init and Supervisor
  03-authority/       # Axiom, Policy, Identity, Keys
  04-capability/      # Capabilities, Process Manager
  05-storage/         # Block Storage, Content Store, Filesystem
  06-network/         # TCP/IP Stack, Drivers
  07-execution/       # Three-Phase Model, Verification
  08-services/        # Terminal, Updates
  09-applications/    # Deterministic Jobs
  10-desktop/         # Window Manager, Compositor
```

The **[Implementation Roadmap](../roadmap/README.md)** defines the task ordering across three phases:
- **Phase 1 (WASM)** - Browser-based development environment
- **Phase 2 (QEMU)** - Virtualized full kernel
- **Phase 3 (Bare Metal)** - Native hardware deployment

---

## Guiding Principle

> **The Axiom defines reality.**  
> **Execution proposes; Policy authorizes; the Axiom commits; effects follow.**

This is the foundation upon which Zero OS is built. No state transition reaches the Axiom without Policy Engine authorization.

---

*Continue to [Background](01-background.md)*
