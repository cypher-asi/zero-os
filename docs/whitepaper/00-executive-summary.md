# Orbital OS — Executive Summary

**Version:** 1.0  
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

Orbital OS introduces a fundamentally different architecture based on six non-negotiable invariants:

| Invariant | Guarantee |
|-----------|-----------|
| **Authoritative History** | Exactly one totally-ordered, hash-chained record of state transitions (the Axiom) |
| **Deterministic Authority** | Control-plane state is a pure function of the Axiom — always reproducible |
| **Parallel Execution** | Full SMP support; nondeterminism allowed in execution, never in authority |
| **No Unauthorized Effects** | No externally visible side effect without explicit Axiom authorization |
| **Crash Safety** | Pre-commit work is discardable; post-commit effects are idempotent and retryable |
| **Verifiable Computation** | Any authoritative result can be independently verified via replay |

---

## Key Differentiators

### vs. Verified Microkernels (seL4)
seL4 proves the kernel is correct. Orbital proves the **system behaved correctly**. The Axiom provides what seL4 lacks: an authoritative history of every meaningful state transition.

### vs. Unix/Linux
Linux logs are advisory artifacts that may be incomplete, reordered, or lost. The Orbital Axiom is the **source of truth** — if it's not in the Axiom, it didn't happen.

### vs. Urbit
Urbit sacrifices parallelism for determinism (single-threaded execution). Orbital achieves **both**: parallel execution with deterministic authority through the three-phase action model.

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────────┐
│                        APPLICATIONS                                 │
│                  (Deterministic Jobs v0)                            │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────────┐
│                      USERLAND SERVICES                              │
│                                                                     │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │
│  │  Axiom    │ │  Policy   │ │    FS     │ │  Network  │           │
│  │ Sequencer │ │  Engine   │ │  Service  │ │  Service  │           │
│  └─────┬─────┘ └─────┬─────┘ └───────────┘ └───────────┘           │
│        │             │                                              │
│  ┌─────┴─────────────┴─────┐ ┌───────────┐ ┌───────────┐           │
│  │     authoritative       │ │  Process  │ │ Terminal  │           │
│  │        control          │ │  Manager  │ │  Service  │           │
│  └─────────────────────────┘ └───────────┘ └───────────┘           │
│                                                                     │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ IPC
┌──────────────────────────▼──────────────────────────────────────────┐
│                          KERNEL                                     │
│         (minimal, no policy — enforcement only)                     │
│                                                                     │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │
│  │ Scheduler │ │  Memory   │ │Capability │ │    IPC    │           │
│  │   (SMP)   │ │  Manager  │ │ Enforcer  │ │Primitives │           │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘           │
│                                                                     │
│  ┌───────────┐                                                      │
│  │ Interrupt │                                                      │
│  │  Handler  │                                                      │
│  └───────────┘                                                      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Where Does Policy Live?

The **Policy Engine** is a user-space service that:
- Defines authorization rules (who can do what)
- Evaluates capability requests
- Provides policy decisions to the Axiom Sequencer

The kernel **enforces** capabilities but does not **decide** policy. All policy logic lives in user space, is auditable, and its decisions are recorded in the Axiom.

---

## The Axiom

The Axiom is the **authoritative record** of system reality:

- **Append-only**: History cannot be rewritten
- **Totally ordered**: Every entry has a definite position
- **Hash-chained**: Integrity is cryptographically guaranteed
- **Crash-consistent**: Survives unexpected termination

Only semantic state transitions enter the Axiom — policy changes, job completions, filesystem transactions, network authorizations. High-frequency runtime events stay out.

---

## Three-Phase Action Model

Every meaningful system action follows this lifecycle:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  PHASE 1    │────▶│  PHASE 2    │────▶│  PHASE 3    │
│  Pre-Commit │     │   Commit    │     │   Effect    │
│  (Propose)  │     │  (Decide)   │     │(Materialize)│
└─────────────┘     └─────────────┘     └─────────────┘
      │                   │                   │
   Tentative          Axiom entry         Idempotent
   execution          accepted            side effects
```

**Crash guarantees:**
- Crash before commit → proposal discarded (no effect)
- Crash after commit → effects retried (idempotent)

---

## Target Platforms

Orbital OS supports multiple deployment targets:

| Platform | Purpose | Status |
|----------|---------|--------|
| **Hosted mode** | Development and debugging — runs as Rust binary on host OS | Development |
| **QEMU** | Integration testing and production virtualized workloads | Production |
| **Bare metal** | Native x86_64 hardware deployment | Production |

For production deployments, both QEMU (virtualized) and bare metal are supported. Hosted mode is intended for development iteration only.

---

## Application Model (v0)

Version 0 supports **deterministic applications only**:

- Execute as discrete jobs with explicit inputs/outputs
- Content-addressed inputs and outputs
- Pinned execution environment
- No hidden nondeterminism (wall-clock, implicit randomness, etc.)
- Results are replay-verifiable

Interactive, long-running, nondeterministic applications are reserved for future versions.

### Determinism Enforcement

Orbital enforces determinism through multiple layers:

| Layer | Mechanism |
|-------|-----------|
| **Static Analysis** | The `orbital-lint` tool analyzes Rust code at compile time, detecting nondeterministic patterns (time access, random calls, unsafe concurrency) |
| **Runtime Sandbox** | Syscall filtering blocks nondeterministic operations; forbidden calls terminate the job |
| **Environment Pinning** | Execution environment is content-addressed; same environment hash guarantees identical behavior |
| **Deterministic Runtime** | The `orbital-rt` runtime library provides deterministic replacements for time, randomness (seeded), and concurrency (fork-join) |

Applications must be built with the Orbital toolchain, which enforces compliance before deployment.

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
- **Core Principles** — The six invariants in detail
- **Architecture Overview** — System structure and component relationships
- **Comparative Analysis** — Technical comparison with seL4, Linux, Plan 9, Urbit
- **Formal Specifications** — Axiom, kernel, services, applications, verification
- **State Machine Diagrams** — Precise behavioral specifications
- **Implementation Roadmap** — Phased development plan

---

## Guiding Principle

> **The Axiom defines reality.**  
> **Execution proposes; commits decide; effects follow.**

This is the foundation upon which Orbital OS is built.

---

*Continue to [Background](01-background.md) →*
