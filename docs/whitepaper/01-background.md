# Orbital OS - Background

**Version:** 2.0  
**Status:** Whitepaper  
**Classification:** Public

---

## 1. The Rise of Autonomous AI Systems

### 1.1 From Tools to Autonomous Agents

Artificial intelligence systems are rapidly evolving from decision-support tools into **autonomous agents** capable of planning, acting, coordinating, and executing tasks with minimal or no human intervention. These autonomous AI systems increasingly operate across critical domains including financial markets, infrastructure management, cybersecurity, logistics, communications, and governance.

Unlike traditional software, autonomous AI systems:

- **Continuously observe and act** on the world
- **Operate over long time horizons** with persistent state and memory
- **Adapt their behavior** based on feedback and learning
- **Interact with other autonomous systems** at machine timescales

As autonomy increases, the locus of control shifts away from humans and toward software-defined decision loops. This shift fundamentally changes the risk profile of AI deployment. Errors, misalignment, or exploitation are no longer isolated incidents; they can **compound, propagate, and escalate** without immediate human oversight.

### 1.2 The Distributed Nature of Modern AI

Modern autonomous AI systems are not monolithic models. They are **distributed systems** composed of:

| Component | Role |
|-----------|------|
| **Multiple models and planners** | Decision-making across different modalities and time horizons |
| **Orchestration layers and schedulers** | Coordinating tasks, managing workflows |
| **Memory and state stores** | Maintaining context, learning from history |
| **External tools, APIs, and data feeds** | Interfacing with the external world |
| **Runtime environments** | Varying trust assumptions and execution contexts |

In this setting, failures often emerge **not from any single component**, but from interactions between components under real-world conditions. Traditional testing and monitoring approaches are insufficient for systems of this complexity.

### 1.3 The Verifiability Imperative

To safely deploy autonomous AI at scale, systems must be **verifiable by design**. Verifiability enables operators, regulators, and other systems to:

- **Deterministically reproduce** system behavior given the same inputs and state
- **Audit decision paths** and action histories
- **Establish cryptographic guarantees** over identity, execution, and state transitions
- **Reason about system invariants** even as behavior adapts over time

**Without verifiability, autonomous AI systems become opaque actors embedded in critical infrastructure** - difficult to debug, impossible to audit, and unsafe to entrust with high-impact responsibilities.

As autonomy increases, verifiability transitions from a desirable property to a **prerequisite for stability and trust**.

---

## 2. Autonomous AI, Adversarial Pressure, and Secure Execution

### 2.1 The Expanding Attack Surface

The rise of autonomous AI systems coincides with an increasingly adversarial digital environment. Autonomous agents expand the attack surface of modern infrastructure by:

- **Operating continuously and unattended** - no human in the loop to notice anomalies
- **Holding long-lived credentials and secrets** - attractive targets for compromise
- **Making real-time decisions** that can be exploited or manipulated
- **Interfacing with external systems** in complex, often under-specified ways

### 2.2 AI as Force Multiplier for Attackers

AI itself is becoming a force multiplier for attackers. Automated vulnerability discovery, exploit generation, phishing, social engineering, and adaptive malware are now AI-assisted. This creates a feedback loop in which **insecure autonomous systems can be compromised faster, at greater scale, and with fewer human resources** than ever before.

### 2.3 The AGI Horizon

As systems approach artificial general intelligence (AGI), these pressures intensify. AGI-capable systems will require:

| Requirement | Implication |
|-------------|-------------|
| **Broad access to tools and data** | Larger attack surface, more resources to protect |
| **Persistent memory and internal state** | Long-lived context that must remain secure |
| **The ability to reason, plan, and self-modify** | Dynamic behavior that resists static analysis |
| **Authority to act across multiple domains** | Cross-cutting capabilities that must be governed |

In such systems, **traditional security models break down**. Perimeter defenses, implicit trust, and reactive patching are insufficient when the system itself is a powerful autonomous actor.

### 2.4 Structural Guarantees for AGI-Aligned Systems

Instead, AGI-aligned systems must be built atop **strong structural guarantees**:

| Guarantee | Description |
|-----------|-------------|
| **Type-safe and memory-safe foundations** | Entire classes of vulnerabilities eliminated by construction. Safety enforced at the language, runtime, and kernel level - not retrofitted after deployment. |
| **Deterministic and auditable execution paths** | Critical decision logic is replayable and inspectable. Non-determinism is isolated, bounded, and explicitly accounted for. |
| **Secure enclaves and isolated execution** | Long-lived secrets - cryptographic keys, identity credentials, model weights, signing authorities - protected from both external compromise and internal misuse. Accessible only through narrowly scoped, policy-governed interfaces. |
| **Capability-based authority and least privilege** | Autonomous agents operate with explicit, revocable capabilities rather than ambient access. Authority is delegated intentionally and logged immutably. |
| **Tamper-evident logs and cryptographic provenance** | Actions, decisions, and state transitions recorded in append-only, verifiable logs. Enables accountability, forensic analysis, and coordinated response across distributed systems. |

**As AI systems become increasingly autonomous, security and safety converge with systems engineering.** The question is no longer how to secure AI systems after they are built, but how to build systems that remain safe, stable, and governable as intelligence and autonomy increase.

---

## 3. The Crisis of System Opacity

Modern operating systems are **black boxes**. Despite decades of engineering, the fundamental question - *"What did the system actually do?"* - remains unanswerable with certainty.

### 3.1 The Debugging Problem

When a production system fails:

- **Logs are scattered** across files, services, and machines
- **Timestamps may drift** or conflict between components
- **Log levels filter** critical information at the source
- **Rotation policies** may have deleted relevant entries
- **Correlation requires** manual forensic reconstruction

The result: debugging distributed systems is an exercise in **archaeology**, not engineering.

### 3.2 The Audit Problem

Security audits of modern systems rely on:

- **Sampling** - examining a fraction of system behavior
- **Heuristics** - detecting patterns that "look suspicious"
- **Trust assumptions** - believing logs weren't tampered with
- **Point-in-time snapshots** - missing transient states

No mainstream OS can answer: *"Prove this file was only accessed by authorized processes."*

### 3.3 The Crash Recovery Problem

When systems crash:

- **Filesystems** use journaling to achieve *structural* consistency
- **Databases** use WAL to achieve *data* consistency
- **Applications** implement their own recovery (or don't)
- **The OS itself** provides no semantic crash recovery

The guarantee is "probably consistent" - not "definitely correct."

### 3.4 The Verification Problem

Modern software increasingly requires:

- **Reproducible builds** - same source > same binary
- **Supply chain verification** - prove provenance of components
- **Computation verification** - prove a result was computed correctly

No mainstream OS provides infrastructure for verifiable computation at the system level.

---

## 4. Why Existing Approaches Fall Short

### 4.1 The Microkernel Approach (seL4, L4, QNX, Mach)

**What they achieve:**
- Minimal kernel with reduced attack surface
- Formal verification of kernel correctness (seL4)
- Strong process isolation
- Capability-based security

**What they lack:**

| Gap | Consequence |
|-----|-------------|
| No authoritative history | Cannot prove what the system did |
| Application behavior is opaque | Verification stops at kernel boundary |
| No semantic crash recovery | Applications must implement their own |
| No verifiable computation | Results cannot be independently validated |

**seL4 proves the kernel won't misbehave. It cannot prove the system behaved correctly.**

### 4.2 The Unix/POSIX Approach (Linux, BSD, macOS)

**What they achieve:**
- Mature, battle-tested codebases
- Vast hardware support
- Rich ecosystem of tools and applications
- Strong community and documentation

**What they lack:**

| Gap | Consequence |
|-----|-------------|
| Logs are advisory | May be incomplete, reordered, or falsified |
| Monolithic trust model | Root can do anything without audit |
| No deterministic state derivation | Cannot replay to verify behavior |
| Fire-and-forget syscalls | No transaction semantics |
| Filesystem-level crash recovery only | Application state may be inconsistent |

**Linux cannot answer: "Given this log, reconstruct the exact system state."**

### 4.3 The Plan 9 Approach

**What they achieve:**
- Everything is a file (extreme composability)
- Per-process namespaces (isolation without containers)
- Network transparency
- User-space services

**What they lack:**

| Gap | Consequence |
|-----|-------------|
| Mutable namespaces | No authoritative history of namespace changes |
| No transaction semantics | Operations are not atomic across services |
| No verification infrastructure | Same limitations as Unix |

**Plan 9 has the right service architecture but lacks the authoritative spine.**

### 4.4 Blockchain Virtual Machines (Ethereum, Solana, etc.)

**What they achieve:**
- Deterministic execution guarantees
- Global verifiable state
- Cryptographic integrity
- Smart contract composability

**What they lack:**

| Gap | Consequence |
|-----|-------------|
| Global consensus overhead | Latency measured in seconds or minutes |
| Gas/fee economics | Every operation has a cost |
| Limited computation model | Not designed for general-purpose workloads |
| Abstract VMs only | Cannot run on bare metal or utilize real hardware |
| No local authority | Every state change requires network consensus |

**Blockchain VMs prove global verifiable state is possible. They prove nothing about local, high-performance computation.**

### 4.5 Urbit

**What they achieve:**
- Deterministic event log (the "event log")
- Reproducible state derivation
- Persistent identity model
- Self-contained personal server

**What they lack:**

| Gap | Consequence |
|-----|-------------|
| Single-threaded execution | Cannot utilize modern multi-core CPUs |
| Novel, esoteric stack | Steep learning curve (Hoon, Nock, Arvo) |
| Limited ecosystem | Few applications, small community |
| Cannot leverage existing systems | No POSIX compatibility, no standard tooling |
| Custom language requirement | Must rewrite everything in Hoon |
| No hardware abstraction | Runs as a VM on top of Unix |

**Urbit proves that deterministic personal computing is achievable. But its complexity, inscrutability, and single-threaded model make it unsuitable as a foundation for general-purpose, high-performance systems.**

---

## 5. The Orbital Insight

### 5.1 Separating Execution from Authority

The critical insight behind Orbital OS is that **operating system state should be derived from authority, not from execution**.

In traditional operating systems, state is the accumulated result of every syscall, every interrupt, every scheduling decision. This execution-derived state is:
- **Nondeterministic** - depends on timing, ordering, hardware quirks
- **Unreproducible** - cannot be reconstructed from any record
- **Opaque** - no way to verify what sequence of events produced it

Orbital introduces an **authority layer** between execution and state:

```
+-----------------------------------------------------------+
|                    EXECUTION REALM                        |
|   (parallel, nondeterministic, speculative)               |
|                                                           |
|   Threads run, interrupts fire, caches miss...            |
|   All of this is TENTATIVE                                |
+-----------------------------+-----------------------------+
                              | proposals
                              v
+-----------------------------------------------------------+
|                    POLICY ENGINE                          |
|              (Gatekeeper of Authority)                    |
|                                                           |
|   ALL proposals MUST pass through policy evaluation       |
|   Authenticates identity, evaluates rules, decides        |
+-----------------------------+-----------------------------+
                              | authorized proposals only
                              v
+-----------------------------------------------------------+
|                    AUTHORITY LAYER                        |
|                      (The Axiom)                          |
|                                                           |
|   Single, totally-ordered, hash-chained sequence          |
|   This is the ONLY source of truth                        |
|   ONLY accepts policy-approved entries                    |
+-----------------------------+-----------------------------+
                              | deterministic reduction
                              v
+-----------------------------------------------------------+
|                   DERIVED STATE                           |
|   (deterministic, reproducible, verifiable)               |
|                                                           |
|   Control-plane state is a PURE FUNCTION of Axiom         |
|   Given same Axiom > always same state                    |
+-----------------------------------------------------------+
```

### 5.2 Why This Distinction Matters

**Nondeterminism in execution is acceptable:**
- Threads can be scheduled in any order
- Interrupts can arrive at any time
- Caches can hit or miss
- Parallel execution can interleave arbitrarily

None of this matters to system correctness, because execution only produces **proposals**.

**Nondeterminism in authority is catastrophic:**
- If the authoritative record varies, the system has no single truth
- If state derivation is nondeterministic, verification is impossible
- If different nodes derive different state from the same history, consensus fails

The Axiom is the authority. Execution is merely speculation.

### 5.3 The Two Realms

| Realm | Characteristics | Examples |
|-------|-----------------|----------|
| **Execution** | Nondeterministic, parallel, observable | Scheduling order, interrupt timing, cache behavior |
| **Authority** | Deterministic, sequential, auditable | Axiom entries, control-plane state, verification |

Execution proposes. Authority decides. Effects follow.

---

## 6. The Policy Engine: Gatekeeper of Authority

**All state transitions must pass through the Policy Engine before reaching the Axiom.** This is not optional, not recommended - it is an architectural invariant. No proposal bypasses policy evaluation.

The Policy Engine is the central point of control for what the system is allowed to do.

### 6.1 Role of the Policy Engine

```
+--------------+     +--------------+     +--------------+
|   Service    |---->|    Policy    |---->|    Axiom     |
|   Request    |     |    Engine    |     |  Sequencer   |
+--------------+     +--------------+     +--------------+
                           |
                    +------+------+
                    | DENY/ALLOW  |
                    +-------------+
```

The Policy Engine:
- **Evaluates every proposal** before it can be submitted to the Axiom
- **Enforces authorization rules** - who can do what, under what conditions
- **Manages capabilities** - granting, attenuating, and revoking access
- **Records all decisions** - policy evaluations are themselves Axiom entries

### 6.2 Policy Decisions Are Authoritative

Unlike traditional OS permission checks (which happen at execution time and are ephemeral), Orbital policy decisions are:

| Property | Description |
|----------|-------------|
| **Recorded** | Every policy decision is logged in the Axiom |
| **Auditable** | Any decision can be traced and explained |
| **Deterministic** | Same policy state + same request > same decision |
| **Versioned** | Policy rules themselves are Axiom entries, enabling rollback |

### 6.3 What Flows Through Policy

**Every consequential action must be authorized:**

| Operation Type | Policy Question |
|----------------|-----------------|
| **Process creation** | Can this identity spawn this process? |
| **Filesystem operations** | Can this identity create/read/write this path? |
| **Network connections** | Can this service connect to this endpoint? |
| **Capability delegation** | Can this identity grant this capability? |
| **System upgrades** | Is this image authorized for activation? |
| **Key usage** | Can this identity use this signing key? |
| **Service lifecycle** | Can this action start/stop this service? |
| **Policy modification** | Can this identity modify these rules? |

**Nothing reaches the Axiom without policy approval. Nothing is approved without a permanent record.**

### 6.4 The Policy-First Guarantee

This architectural decision has profound implications:

1. **Complete audit trail** - Every authorization decision is recorded
2. **Verifiable decisions** - Anyone can verify policy was correctly applied
3. **Deterministic authorization** - Same policy state + same request = same decision
4. **Revocable access** - Capabilities can be revoked and the revocation is enforced
5. **No ambient authority** - Everything requires explicit authorization

---

## 7. Cryptographic Identity and Key Management

Orbital OS treats cryptographic keys as first-class citizens, with deterministic derivation and policy-controlled usage.

### 7.1 The Key Management Problem

Traditional systems handle cryptographic keys poorly:
- Keys are stored as opaque blobs in files or HSMs
- Access control is coarse (you have it or you don't)
- Key usage is not audited at the OS level
- Key derivation is application-specific and inconsistent

### 7.2 Orbital Key Architecture

Orbital implements a **policy-controlled key hierarchy** with deterministic derivation:

```
+-----------------------------------------------------------+
|                    ROOT SEED                              |
|       (secure boundary protected, never exported)         |
+-----------------------------+-----------------------------+
                              | deterministic derivation
                              v
+-----------------------------------------------------------+
|                 KEY DERIVATION SERVICE                    |
|                                                           |
|   path: "/system/axiom/signing"  >  [derived key]         |
|   path: "/user/alice/encryption" >  [derived key]         |
|   path: "/job/{hash}/attestation" > [derived key]         |
|                                                           |
|   Keys are derived on-demand, never stored                |
+-----------------------------------------------------------+
```

### 7.3 Deterministic Key Derivation

Keys are derived using a **deterministic path-based scheme** (similar to BIP-32/BIP-44):

| Component | Purpose |
|-----------|---------|
| **Root Seed** | Master secret, protected within secure boundary |
| **Derivation Path** | Hierarchical key identifier (e.g., `/system/axiom/signing`) |
| **Context** | Additional binding data (Axiom hash, timestamp) |
| **Derived Key** | Deterministically computed, never stored |

This means:
- **Reproducibility** - Same seed + path > same key, always
- **No key storage** - Keys are derived when needed, discarded after use
- **Hierarchical access** - Access to `/user/alice/*` doesn't grant `/user/bob/*`

### 7.4 Policy-Controlled Signing

Every signing operation must be authorized by the Policy Engine:

```rust
struct SigningRequest {
    /// The data to sign
    payload: Hash,
    
    /// Which key path to use
    key_path: KeyPath,
    
    /// Who is requesting
    requestor: ServiceId,
    
    /// Why (links to authorizing Axiom entry)
    authorization: AxiomRef,
}

// Policy Engine evaluates:
// 1. Does requestor have capability for this key_path?
// 2. Is the authorization entry valid and committed?
// 3. Does the payload match what was authorized?
// > If all pass: derive key, sign, record usage, return signature
```

### 7.5 Key Usage Audit Trail

All key operations are recorded:

| Event | Recorded Data |
|-------|---------------|
| **Key derivation** | Path, requestor, authorization, timestamp |
| **Signing** | Payload hash, key path, signature, authorization |
| **Encryption** | Target identity, key path, authorization |
| **Key rotation** | Old path, new path, reason, authorization |

This enables complete forensic reconstruction: *"Which keys were used, by whom, for what, under whose authority?"*

### 7.6 Secure Boundary

The Key Derivation Service operates within a **software-defined secure boundary** that is always enforced, regardless of platform. Hardware protection is optional but recommended when available.

**Software Secure Boundary (Always Present):**
- Isolated address space with no shared memory
- Capability-gated IPC - only authorized services can request key operations
- All requests validated against Policy Engine before execution
- Derived keys exist only in memory, zeroed immediately after use
- No key material ever written to persistent storage

**Hardware Enhancement (Optional):**

| Platform | Optional Enhancement |
|----------|---------------------|
| **Bare metal** | TPM 2.0 sealed storage for root seed |
| **QEMU** | Emulated vTPM for testing hardware flows |
| **Hosted** | None - software boundary is sufficient for development |

The software secure boundary provides the security guarantee. Hardware protection adds defense-in-depth where available, but is not required for correct operation.

---

## 8. What Orbital OS Aims to Achieve

### 8.1 Primary Objectives

| Objective | Description |
|-----------|-------------|
| **Verifiable behavior** | Any system state transition can be audited with cryptographic proof |
| **Deterministic authority** | Control-plane state is a pure function of history |
| **Crash safety by construction** | Not a convention - architecturally guaranteed |
| **Parallel execution** | Full utilization of modern multi-core hardware |
| **Real-world capability** | Run actual workloads on actual machines |
| **Policy-controlled operations** | All consequential actions require explicit authorization |
| **Secure key management** | Cryptographic operations are audited and policy-controlled |

### 8.2 Non-Objectives (Explicitly Out of Scope)

| Non-Objective | Rationale |
|---------------|-----------|
| Global consensus | Adds latency without local benefit; reserved for multi-node replication |
| Deterministic scheduling | Unnecessary for authority; would cripple performance |
| Universal language support | Rust first; others only under strict determinism |
| Backward compatibility | Clean-slate design; no POSIX baggage |

---

## 9. Core Design Principles

### 9.1 The Axiom Is Reality

The Axiom is not a log. It is not a journal. It is not a record of what happened.

**The Axiom defines what happened.**

If an event is not in the Axiom, it did not occur - regardless of what execution believed.

### 9.2 Execution Is Speculation

All execution is tentative until committed:
- Work may be performed in parallel
- Results may be computed speculatively
- Side effects may be prepared but not finalized

Nothing is real until the Axiom says so.

### 9.3 Authority Is Pure

Control-plane state (who can do what, what exists, what is authorized) is derived exclusively from the Axiom through pure, deterministic reduction.

Given the same Axiom, every node will derive the same authority state. Always.

### 9.4 Policy Is Explicit

No operation with external consequences occurs without explicit policy authorization:
- The Policy Engine evaluates every consequential request
- Policy decisions are recorded in the Axiom
- Policy rules are themselves versioned Axiom entries

### 9.5 Effects Are Idempotent

Post-commit effects (I/O, external communication, state materialization) must be:
- **Idempotent** - safe to retry
- **Authorized** - backed by a Axiom entry
- **Receipted** - completion recorded for audit

### 9.6 Crashes Are Expected

The system assumes crashes will happen:
- Pre-commit state is discardable by design
- Post-commit effects are retryable by design
- The Axiom survives by design

There is no "recovery" - only "continuation from last known reality."

### 9.7 Verification Is Mandatory

Any result that becomes authoritative must be verifiable:
- Content-addressed inputs
- Pinned execution environment
- Deterministic computation
- Committed receipt

If it cannot be verified, it cannot be trusted.

### 9.8 Keys Are Derived, Not Stored

Cryptographic keys are:
- Derived deterministically from a protected root seed
- Generated on-demand for specific operations
- Never persisted in their derived form
- Always used under policy control

---

## 10. Architectural Principles

### 10.1 Minimal Kernel

The kernel provides exactly five things:
1. Preemptive multitasking (SMP)
2. Virtual memory and address-space isolation
3. Capability enforcement
4. Fast IPC primitives
5. Interrupt and timer handling

Everything else - filesystems, networking, device drivers, policy - lives in user space.

### 10.2 User-Space Services

The OS is composed of cooperating services:
- Each service is a separate process
- Services communicate via IPC
- Service failure does not crash the kernel
- Services can be updated independently

### 10.3 Capability-Based Security

There is no ambient authority:
- Every operation requires an explicit capability
- Capabilities are unforgeable tokens
- Capabilities can be attenuated (reduced) but not amplified
- The kernel enforces capability checks

### 10.4 Content-Addressed Everything

Immutable artifacts are identified by their content hash:
- System images
- Application binaries
- Job inputs and outputs
- Execution environment specifications

This enables deduplication, caching, integrity verification, and deterministic references.

### 10.5 Immutable System Images

System software is delivered as immutable, signed images:
- No mutation of system files at runtime
- Updates are atomic image swaps
- Rollback is trivial (previous image still exists)
- Mutable state lives outside system images

---

## 11. The Path Forward

Orbital OS is designed to be built incrementally:

1. **Hosted simulator** - Rust binary on existing OS
2. **QEMU kernel** - Minimal kernel in virtual machine
3. **Storage/filesystem** - User-space FS service
4. **Networking** - User-space network stack
5. **Isolation tiers** - Resource limits and sandboxing
6. **Transactional upgrades** - Atomic system updates
7. **Bare metal** - Real hardware deployment
8. **Visual OS** - Deterministic UI layer

Each phase is independently useful and testable.

---

## 12. Summary

| Problem | Existing Approach | Orbital Approach |
|---------|-------------------|------------------|
| What happened? | Scattered logs, archaeology | Single authoritative Axiom |
| Was it correct? | Trust assumptions | Verifiable replay |
| Did it crash safely? | Probably, maybe | Guaranteed by construction |
| Can we parallelize? | Yes (Unix) or No (Urbit) | Yes, with deterministic authority |
| Can we verify? | Manual audit | Cryptographic proof |
| Who authorized it? | Hope the logs are complete | Policy Engine recorded every decision |
| How were keys used? | Application-specific, opaque | Deterministic derivation, full audit trail |
| Can autonomous AI run safely? | Retrofit security, implicit trust | Structural guarantees, capability-based authority |
| Is it AGI-ready? | Perimeter defenses, reactive patching | Secure enclaves, tamper-evident logs, least privilege |

**Orbital OS is not an incremental improvement. It is a fundamental rearchitecture of what an operating system can guarantee.**

**In an era of increasingly autonomous AI systems, Orbital provides the verifiable substrate that makes safe deployment of intelligent agents possible.**

---

*[Executive Summary](00-executive-summary.md) | [Core Principles](02-core-principles.md)*
