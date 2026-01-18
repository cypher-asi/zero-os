# Permissions Service

> Policy enforcement for capability grants and resource access.

## Overview

The Permissions Service provides:

1. **Policy Enforcement**: Decide which capabilities can be granted to whom
2. **Audit Logging**: Query the Axiom log for capability history
3. **Permission Queries**: Check if an operation is allowed

This is the *policy* layer. The kernel (Axiom) handles *mechanism* (capability checking). The Permissions Service implements *policy* (should this grant happen?).

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Permissions Service                             │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Policy Database                              ││
│  │                                                                ││
│  │  Rules:                                                        ││
│  │  • Apps can request: [storage-ro, network]                     ││
│  │  • Services can request: [storage-rw, spawn]                   ││
│  │  • Terminal can grant: [console] to children                   ││
│  │  • Storage cannot grant: [network]                             ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Axiom Log Query                              ││
│  │                                                                ││
│  │  • Who granted cap X?                                          ││
│  │  • What caps does process Y hold?                              ││
│  │  • History of capability transfers                             ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Message Handlers:                                                   │
│  • CHECK_PERMISSION → check if grant is allowed                     │
│  • QUERY_CAPS       → list capabilities for a process              │
│  • QUERY_HISTORY    → capability grant history                     │
│  • UPDATE_POLICY    → modify policy rules (admin only)             │
└─────────────────────────────────────────────────────────────────────┘
```

## IPC Protocol

### Permission Check

```rust
/// Check permission request.
pub const MSG_CHECK_PERM: u32 = 0x5000;
/// Check permission response.
pub const MSG_CHECK_PERM_RESPONSE: u32 = 0x5001;

/// Permission check request.
#[derive(Clone, Debug)]
pub struct PermissionCheckRequest {
    /// Process requesting the capability
    pub requester: ProcessId,
    /// Capability being requested
    pub capability_type: String,
    /// Requested permissions
    pub permissions: Permissions,
    /// Context (why is this being requested)
    pub context: Option<String>,
}

/// Permission check response.
#[derive(Clone, Debug)]
pub struct PermissionCheckResponse {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Reason if denied
    pub reason: Option<String>,
    /// Suggested alternative if denied
    pub alternative: Option<String>,
}
```

### Capability Query

```rust
/// Query capabilities request.
pub const MSG_QUERY_CAPS: u32 = 0x5002;
/// Query capabilities response.
pub const MSG_QUERY_CAPS_RESPONSE: u32 = 0x5003;

/// Capability query.
pub struct CapQuery {
    /// Process to query (or None for self)
    pub pid: Option<ProcessId>,
}

/// Capability info.
pub struct CapQueryResult {
    pub capabilities: Vec<CapInfo>,
}

pub struct CapInfo {
    pub slot: CapSlot,
    pub object_type: ObjectType,
    pub object_id: u64,
    pub permissions: Permissions,
    pub granted_by: ProcessId,
    pub granted_at: u64,
}
```

### History Query

```rust
/// Query history request.
pub const MSG_QUERY_HISTORY: u32 = 0x5004;
/// Query history response.
pub const MSG_QUERY_HISTORY_RESPONSE: u32 = 0x5005;

/// History query filter.
pub struct HistoryQuery {
    /// Filter by actor (who did the action)
    pub actor: Option<ProcessId>,
    /// Filter by operation type
    pub operation: Option<CapOperationType>,
    /// Time range (nanos since boot)
    pub from_time: Option<u64>,
    pub to_time: Option<u64>,
    /// Maximum entries to return
    pub limit: usize,
}

/// History entry.
pub struct HistoryEntry {
    pub seq: u64,
    pub timestamp: u64,
    pub actor: ProcessId,
    pub operation: CapOperation,
}
```

## Policy Model

Policies define what capabilities can be granted to whom:

```rust
/// A permission policy rule.
#[derive(Clone, Debug)]
pub struct PolicyRule {
    /// Rule identifier
    pub id: String,
    /// Process class this rule applies to
    pub applies_to: ProcessClass,
    /// Capability types this rule covers
    pub capability_types: Vec<String>,
    /// Whether the action is allowed
    pub allowed: bool,
    /// Required conditions
    pub conditions: Vec<Condition>,
}

/// Process classification.
#[derive(Clone, Debug)]
pub enum ProcessClass {
    /// System services (init, terminal, etc.)
    System,
    /// Runtime services (storage, network, etc.)
    Runtime,
    /// User applications
    Application,
    /// Specific process by name pattern
    Named(String),
    /// Specific process ID
    Pid(ProcessId),
}

/// Condition for a policy rule.
#[derive(Clone, Debug)]
pub enum Condition {
    /// Requester must hold specific capability
    RequesterHolds(String),
    /// Parent process must be specific class
    ParentIs(ProcessClass),
    /// Grant must attenuate permissions
    MustAttenuate,
    /// Time-based restriction
    TimeWindow { start: u64, end: u64 },
}
```

### Example Policy

```rust
// Default policy rules
fn default_policy() -> Vec<PolicyRule> {
    vec![
        // System services can spawn and grant
        PolicyRule {
            id: "system-spawn".to_string(),
            applies_to: ProcessClass::System,
            capability_types: vec!["spawn".to_string()],
            allowed: true,
            conditions: vec![],
        },
        
        // Applications can request storage (read-only)
        PolicyRule {
            id: "app-storage-ro".to_string(),
            applies_to: ProcessClass::Application,
            capability_types: vec!["storage".to_string()],
            allowed: true,
            conditions: vec![
                Condition::MustAttenuate,  // Can't get write if didn't have it
            ],
        },
        
        // Applications can request network
        PolicyRule {
            id: "app-network".to_string(),
            applies_to: ProcessClass::Application,
            capability_types: vec!["network".to_string()],
            allowed: true,
            conditions: vec![],
        },
        
        // Terminal can grant console to children
        PolicyRule {
            id: "terminal-console".to_string(),
            applies_to: ProcessClass::Named("terminal".to_string()),
            capability_types: vec!["console".to_string()],
            allowed: true,
            conditions: vec![
                Condition::ParentIs(ProcessClass::Named("terminal".to_string())),
            ],
        },
        
        // Deny storage service granting network (isolation)
        PolicyRule {
            id: "storage-no-network".to_string(),
            applies_to: ProcessClass::Named("storage".to_string()),
            capability_types: vec!["network".to_string()],
            allowed: false,
            conditions: vec![],
        },
    ]
}
```

## Permission Checking

```rust
impl PermissionsService {
    fn check_permission(&self, request: &PermissionCheckRequest) -> PermissionCheckResponse {
        // 1. Classify the requester
        let class = self.classify_process(request.requester);
        
        // 2. Find applicable rules
        let rules: Vec<_> = self.policy.iter()
            .filter(|r| self.rule_applies(r, &class, &request.capability_type))
            .collect();
        
        // 3. Evaluate rules (first match wins, deny by default)
        for rule in rules {
            if self.evaluate_conditions(rule, request) {
                if rule.allowed {
                    return PermissionCheckResponse {
                        allowed: true,
                        reason: None,
                        alternative: None,
                    };
                } else {
                    return PermissionCheckResponse {
                        allowed: false,
                        reason: Some(format!("Denied by rule: {}", rule.id)),
                        alternative: self.suggest_alternative(request),
                    };
                }
            }
        }
        
        // No matching rule - deny
        PermissionCheckResponse {
            allowed: false,
            reason: Some("No policy allows this permission".to_string()),
            alternative: None,
        }
    }
    
    fn classify_process(&self, pid: ProcessId) -> ProcessClass {
        let info = self.get_process_info(pid);
        
        match info.name.as_str() {
            "init" | "terminal" | "supervisor" => ProcessClass::System,
            "storage" | "network" | "identity" | "permissions" => ProcessClass::Runtime,
            name if name.starts_with("system-") => ProcessClass::System,
            _ => ProcessClass::Application,
        }
    }
}
```

## Axiom Log Integration

The Permissions Service queries the Axiom log for auditing:

```rust
impl PermissionsService {
    /// Query capability grant history.
    fn query_history(&self, query: &HistoryQuery) -> Vec<HistoryEntry> {
        // Request log range from kernel
        let log_entries = syscall_axiom_query(
            query.from_time,
            query.to_time,
            query.limit,
        );
        
        // Filter by criteria
        log_entries.into_iter()
            .filter(|e| self.matches_query(e, query))
            .map(|e| HistoryEntry {
                seq: e.seq,
                timestamp: e.timestamp,
                actor: e.actor,
                operation: e.operation.clone(),
            })
            .collect()
    }
    
    /// Get full capability provenance (chain of grants).
    fn get_provenance(&self, cap_id: u64) -> Vec<HistoryEntry> {
        let mut chain = Vec::new();
        let mut current_id = cap_id;
        
        loop {
            // Find grant that created this capability
            let grant = self.find_grant_for_cap(current_id);
            
            match grant {
                Some(entry) => {
                    chain.push(entry.clone());
                    
                    // Trace back to source capability
                    if let CapOperation::Grant { source_cap_id, .. } = &entry.operation {
                        current_id = *source_cap_id;
                    } else {
                        break;  // Reached original creation
                    }
                }
                None => break,
            }
        }
        
        chain
    }
}
```

## WASM Implementation

```rust
// permissions_service.rs

#![no_std]
extern crate alloc;
extern crate orbital_process;

use alloc::vec::Vec;
use orbital_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("permissions: starting");
    
    // Load policy from storage or use default
    let policy = load_policy_or_default();
    
    let service_ep = create_endpoint();
    register_service("permissions", service_ep);
    send_ready();
    
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            MSG_CHECK_PERM => handle_check_permission(msg),
            MSG_QUERY_CAPS => handle_query_caps(msg),
            MSG_QUERY_HISTORY => handle_query_history(msg),
            MSG_UPDATE_POLICY => handle_update_policy(msg),
            _ => debug("permissions: unknown message"),
        }
    }
}
```
