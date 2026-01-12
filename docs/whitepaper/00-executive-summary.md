# Orbital OS — Executive Summary

**Version:** 2.0  
**Status:** Whitepaper  
**Classification:** Public

---

## What Is Orbital OS?

Orbital OS is a general-purpose operating system built from first principles with a singular objective: **make system behavior verifiable, auditable, and deterministic in meaning** — while preserving parallel execution, high performance, and native hardware support.

Orbital OS is not a blockchain, not a simulation, not a research-only prototype, and not a purely functional curiosity. It is a **real operating system** intended to run **real workloads** on **real machines**.

---

## The Problem

Modern operating systems are **opaque by design**. When something goes wrong:

- Debugging requires forensic archaeology through scattered logs
- Security audits rely on sampling and heuristics, not proof
- Crash recovery is probabilistic — "probably consistent" is the best guarantee
- There is no authoritative record of what actually happened

The question *"What did the system do?"* has no definitive answer in any mainstream OS today.

---

## The Orbital Solution

Orbital OS introduces a fundamentally different architecture based on seven non-negotiable invariants:

| Invariant | Guarantee |
|-----------|-----------|
| **Authoritative History** | Exactly one totally-ordered, hash-chained record of state transitions (the Axiom) |
| **Deterministic Authority** | Control-plane state is a pure function of the Axiom — always reproducible |
| **Parallel Execution** | Full SMP support; nondeterminism allowed in execution, never in authority |
| **No Unauthorized Effects** | No externally visible side effect without explicit Axiom authorization |
| **Crash Safety** | Pre-commit work is discardable; post-commit effects are idempotent and retryable |
| **Verifiable Computation** | Any authoritative result can be independently verified via replay |
| **Cryptographic Identity** | All principals have policy-controlled cryptographic identities |

**The Central Principle: All consequential state transitions flow through the Policy Engine before reaching the Axiom.**

---

## Architecture by Layer

Orbital OS is organized into distinct layers, from most fundamental to least:

```
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 8: APPLICATIONS                                              │
│    Deterministic Jobs, Visual OS                                    │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 7: USER-FACING SERVICES                                      │
│    Terminal, Update Manager                                         │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 6: EXECUTION INFRASTRUCTURE                                  │
│    Job Scheduler, Job Executor, Effect Materializer                 │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 5: NETWORK & DEVICE                                          │
│    Driver Manager, Network Service                                  │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 4: STORAGE                                                   │
│    Block Storage, Filesystem, Content-Addressed Store               │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 3: PROCESS & CAPABILITY                                      │
│    Capability Service, Process Manager                              │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 2: CORE AUTHORITY                                            │
│    Axiom Sequencer, Policy Engine, KDS, Identity Service            │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 1: BOOTSTRAP                                                 │
│    Supervisor                                                       │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 0: KERNEL                                                    │
│    Scheduler, Memory, Capabilities, IPC, Interrupts                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Layer 0: Kernel (Most Fundamental)

The minimal microkernel provides exactly five services:
- Preemptive multitasking (SMP)
- Virtual memory and address-space isolation
- Capability enforcement
- Fast IPC primitives
- Interrupt and timer handling

**Everything else runs in user space.**

### Layer 1: Bootstrap

The **Supervisor** is the first user-space process:
- Spawned by kernel with full system capabilities
- Boots all other services in dependency order
- Monitors service health, restarts failed services

### Layer 2: Core Authority (The "Authority Spine")

Four critical services form the foundation of system trust:

| Service | Role |
|---------|------|
| **Axiom Sequencer** | Single source of truth — append-only, hash-chained log of all state transitions |
| **Policy Engine** | Central authorization gate — ALL proposals must pass policy before reaching Axiom |
| **Key Derivation Service** | Cryptographic foundation — derives keys, performs signing within secure boundary |
| **Identity Service** | Principal management — users, services, nodes with hierarchical crypto keys |

### Layer 3: Process & Capability

- **Capability Service** — Manages capability delegation and revocation
- **Process Manager** — Creates/destroys processes, enforces resource limits

### Layer 4: Storage

- **Block Storage** — Low-level block device abstraction
- **Filesystem Service** — Namespace, metadata, path resolution
- **Content-Addressed Store** — Immutable content by BLAKE3 hash

### Layer 5: Network & Device

- **Driver Manager** — Loads and manages user-space drivers
- **Network Service** — TCP/IP stack, policy-gated connections

### Layer 6: Execution Infrastructure

- **Job Scheduler & Executor** — Runs deterministic jobs in isolation
- **Effect Materializer** — Executes authorized effects after Axiom commit
- **Verification & Receipts** — Binds inputs to outputs cryptographically

### Layer 7: User-Facing Services

- **Terminal Service** — User interaction, command execution
- **Update Manager** — Atomic system image updates, rollback

### Layer 8: Applications

- **Deterministic Jobs** — Content-addressed inputs → content-addressed outputs
- **Visual OS** — Deterministic UI layer (future)

---

## Key Differentiators

### vs. Verified Microkernels (seL4)
seL4 proves the kernel is correct. Orbital proves the **system behaved correctly**. The Axiom provides what seL4 lacks: an authoritative history of every meaningful state transition.

### vs. Unix/Linux
Linux logs are advisory artifacts that may be incomplete, reordered, or lost. The Orbital Axiom is the **source of truth** — if it's not in the Axiom, it didn't happen.

### vs. Urbit
Urbit sacrifices parallelism for determinism (single-threaded execution). Orbital achieves **both**: parallel execution with deterministic authority through the three-phase action model.

---

## Three-Phase Action Model

Every meaningful system action follows this lifecycle, with **mandatory Policy Engine evaluation**:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  PHASE 1    │────▶│   POLICY    │────▶│  PHASE 2    │────▶│  PHASE 3    │
│  Pre-Commit │     │   ENGINE    │     │   Commit    │     │   Effect    │
│  (Propose)  │     │ (Authorize) │     │  (Decide)   │     │(Materialize)│
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
      │                   │                   │                   │
   Tentative          Identity +          Axiom entry         Idempotent
   execution          policy check        accepted            side effects
```

**Key guarantee: The Axiom only accepts entries that have been authorized by the Policy Engine.**

**Crash guarantees:**
- Crash before commit → proposal discarded (no effect)
- Crash after commit → effects retried (idempotent)

---

## Target Platforms

| Platform | Purpose | Status |
|----------|---------|--------|
| **Hosted mode** | Development and debugging — runs as Rust binary on host OS | Development |
| **QEMU** | Integration testing and production virtualized workloads | Production |
| **Bare metal** | Native x86_64 hardware deployment | Production |

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

The convergence of several factors makes Orbital OS timely:

1. **Rust maturity** — Systems language with memory safety, no GC
2. **Hardware capability** — Modern CPUs can afford the three-phase overhead
3. **Security demands** — Supply-chain attacks require verifiable builds
4. **Trust verification** — Users demand transparency; "trust me" is no longer acceptable
5. **AI integration** — Verifiable computation is essential to harness AI-assisted systems safely

---

## What This Whitepaper Contains

This document suite provides:

- **Background & Motivation** — Why existing approaches fall short
- **Core Principles** — The seven invariants in detail
- **Architecture by Layer** — System structure from kernel to applications
- **Comparative Analysis** — Technical comparison with seL4, Linux, Plan 9, Urbit

The accompanying **Specifications** provide formal definitions organized by layer:

```
specs/
├── 00-kernel/          # Kernel, processes, scheduling
├── 01-boot/            # Supervisor
├── 02-authority/       # Axiom, Policy, Identity, Keys
├── 03-capability/      # Capabilities, Process Manager
├── 04-storage/         # Filesystem, Block Storage
├── 05-network/         # Networking, Drivers
├── 06-execution/       # Three-Phase Model, Verification
├── 07-services/        # Terminal, Updates
└── 08-applications/    # Application Model, Visual OS
```

---

## Guiding Principle

> **The Axiom defines reality.**  
> **Execution proposes; Policy authorizes; the Axiom commits; effects follow.**

This is the foundation upon which Orbital OS is built. No state transition reaches the Axiom without Policy Engine authorization.

---

*Continue to [Background](01-background.md) →*
