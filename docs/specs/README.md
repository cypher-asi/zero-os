# Orbital OS Specifications

**Version:** 2.0  
**Status:** Specification Index

---

## Overview

The Orbital OS specifications are organized by **architectural layer**, from most fundamental (Layer 0) to least fundamental (Layer 8). Each layer depends only on layers below it.

---

## Layer Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 8: APPLICATIONS                                              │
│    Deterministic Jobs, Visual OS                                    │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 7: USER-FACING SERVICES                                      │
│    Terminal Service, Update Manager                                 │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 6: EXECUTION INFRASTRUCTURE                                  │
│    Three-Phase Action Model, Verification & Receipts                │
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
│    Axiom Sequencer, Policy Engine, Key Service, Identity Service    │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 1: BOOTSTRAP                                                 │
│    Supervisor                                                       │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 0: KERNEL                                                    │
│    Scheduler (SMP), Memory Manager, Capability Enforcer,            │
│    IPC Primitives, Interrupt Handler                                │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Specification Index

### Layer 0: Kernel (Most Fundamental)

The minimal microkernel provides hardware abstraction and process isolation.

| Specification | Description |
|---------------|-------------|
| [Kernel](00-kernel/01-kernel.md) | Core kernel services: scheduler, memory, capabilities, IPC, interrupts |
| [Processes and Scheduling](00-kernel/02-processes.md) | Process model, thread model, scheduling algorithm |

---

### Layer 1: Bootstrap

First user-space process (PID 1), handling boot and runtime supervision.

| Specification | Description |
|---------------|-------------|
| [Init](01-boot/01-init.md) | Boot sequence, kernel handoff, service startup |
| [Supervisor](01-boot/02-supervisor.md) | Runtime health monitoring, restart management, shutdown |

---

### Layer 2: Core Authority

The "authority spine" — these services must start first as all others depend on them.

| Specification | Description |
|---------------|-------------|
| [Axiom](02-authority/01-axiom.md) | Append-only, hash-chained log of all state transitions |
| [Policy Engine](02-authority/02-policy.md) | Central authorization gate for all operations |
| [Key Derivation Service](02-authority/03-keys.md) | Key derivation, signing, encryption within secure boundary |
| [Identity Service](02-authority/04-identity.md) | Principal management, credentials, authentication |

---

### Layer 3: Process & Capability

Access control and process lifecycle management.

| Specification | Description |
|---------------|-------------|
| [Capability Service](03-capability/01-capabilities.md) | Capability delegation, revocation, access control |
| [Process Manager](03-capability/02-processes.md) | Process spawn/kill, resource limits, lifecycle |

---

### Layer 4: Storage

Persistent storage and content addressing.

| Specification | Description |
|---------------|-------------|
| [Filesystem and Storage](04-storage/01-filesystem.md) | Block storage, filesystem namespace, content-addressed store |

---

### Layer 5: Network & Device

Device drivers and network stack.

| Specification | Description |
|---------------|-------------|
| [Networking](05-network/01-networking.md) | TCP/IP stack, socket management, policy-gated connections |

---

### Layer 6: Execution Infrastructure

The three-phase action model and verification system.

| Specification | Description |
|---------------|-------------|
| [Three-Phase Action Model](06-execution/01-three-phase.md) | Proposal → Policy → Commit → Effect lifecycle |
| [Verification and Receipts](06-execution/02-verification.md) | Deterministic verification, cryptographic receipts |

---

### Layer 7: User-Facing Services

Services that directly interact with users.

| Specification | Description |
|---------------|-------------|
| [Terminal](07-services/01-terminal.md) | User interaction, command execution |
| [Update Manager](07-services/02-update-manager.md) | System image updates, atomic swaps, rollback |

---

### Layer 8: Applications

User applications and visual interface.

| Specification | Description |
|---------------|-------------|
| [Application Model](08-applications/01-application-model.md) | Deterministic jobs, inputs/outputs, verification |
| [Visual OS](08-applications/02-visual-os.md) | Deterministic UI layer (future) |

---

## Reading Order

For a complete understanding, read specifications in layer order:

1. **Layer 0** — Understand the kernel foundation
2. **Layer 1** — Understand how services are bootstrapped
3. **Layer 2** — Understand the authority model (Axiom + Policy)
4. **Layer 3** — Understand capabilities and process management
5. **Layers 4-8** — Understand specific subsystems

Alternatively, for a specific topic:

- **Security**: Layer 2 (Policy, Identity) → Layer 3 (Capabilities)
- **Storage**: Layer 4 (Filesystem)
- **Applications**: Layer 8 → Layer 6 (Three-Phase Model)

---

## Key Concepts

### The Axiom

The single source of truth — an append-only, hash-chained log of all meaningful state transitions.

### Policy Engine

Central authorization gate. **All proposals must pass through the Policy Engine before reaching the Axiom.**

### Three-Phase Action Model

Every meaningful action follows:
1. **Phase 1**: Pre-commit (prepare proposal)
2. **Policy Gate**: Authorization check
3. **Phase 2**: Commit (Axiom accepts)
4. **Phase 3**: Effect materialization

### Capabilities

Unforgeable tokens that grant specific permissions. Can be delegated with attenuation, never amplification.

---

## Cross-References

| Topic | Primary Spec | Related Specs |
|-------|--------------|---------------|
| Process creation | Layer 3 | Layer 0 (kernel), Layer 2 (policy) |
| File operations | Layer 4 | Layer 2 (policy), Layer 3 (capabilities) |
| Network connections | Layer 5 | Layer 2 (policy, identity) |
| Job execution | Layer 8 | Layer 6 (three-phase), Layer 4 (content store) |

---

*[← Whitepaper](../whitepaper/00-executive-summary.md)*
