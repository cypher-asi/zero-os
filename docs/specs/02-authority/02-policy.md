# Policy Engine Specification

**Version:** 1.0  
**Status:** Specification  
**Component:** Layer 2 - Core Authority

---

## 1. Overview

The Policy Engine is the **central authorization service** in Orbital OS. Every consequential operation must be authorized by the Policy Engine before it can affect system state.

### 1.1 Position in Architecture

| Layer | Component | Relationship |
|-------|-----------|--------------|
| Layer 0 | Kernel | Enforces capabilities granted via policy |
| Layer 1 | Supervisor | Starts Policy Engine early in boot |
| **Layer 2** | **Policy Engine** | **Central authorization gate** |
| Layer 2 | Axiom Sequencer | Only accepts policy-approved proposals |
| Layer 2+ | All Services | Must request authorization from Policy Engine |

### 1.2 Core Invariant

> **All proposals must pass through the Policy Engine before reaching the Axiom Sequencer. No state transition bypasses policy evaluation.**

---

## 2. Policy Engine Interface

### 2.1 Service Interface

```rust
/// Policy Engine service interface
pub trait PolicyEngineService {
    /// Evaluate an authorization request
    fn evaluate(
        &self,
        request: PolicyRequest,
    ) -> Result<PolicyDecision, PolicyError>;
    
    /// Get current policy state (for debugging/audit)
    fn get_policy_state(&self) -> Result<PolicyStateSnapshot, PolicyError>;
    
    /// Add a policy rule (requires appropriate capability)
    fn add_rule(
        &mut self,
        rule: PolicyRule,
        authorization: AuthToken,
    ) -> Result<RuleId, PolicyError>;
    
    /// Remove a policy rule
    fn remove_rule(
        &mut self,
        rule_id: RuleId,
        authorization: AuthToken,
    ) -> Result<(), PolicyError>;
    
    /// List policy rules
    fn list_rules(&self, filter: RuleFilter) -> Result<Vec<PolicyRule>, PolicyError>;
}
```

### 2.2 Policy Request

```rust
/// Request to the Policy Engine
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyRequest {
    /// Who is making the request
    pub requestor: Identity,
    
    /// What action is being requested
    pub action: PolicyAction,
    
    /// On what resource
    pub resource: ResourceRef,
    
    /// Additional context
    pub context: PolicyContext,
    
    /// Request signature (for verification)
    pub signature: Option<Signature>,
}

/// Context for policy evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyContext {
    /// Current Axiom sequence number
    pub axiom_sequence: u64,
    
    /// Requesting service identity
    pub service: ServiceId,
    
    /// Process making the request
    pub process: ProcessId,
    
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}
```

### 2.3 Policy Actions

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyAction {
    // Process operations
    CreateProcess { image: ContentHash },
    TerminateProcess { pid: ProcessId },
    
    // Filesystem operations
    CreateFile { path: PathBuf, file_type: FileType },
    ReadFile { path: PathBuf },
    WriteFile { path: PathBuf },
    DeleteFile { path: PathBuf },
    
    // Network operations
    Connect { address: SocketAddr },
    Listen { address: SocketAddr },
    
    // Key operations
    Sign { key_path: KeyPath, message_hash: Hash },
    Encrypt { key_path: KeyPath },
    Decrypt { key_path: KeyPath },
    
    // Identity operations
    CreateIdentity { parent: IdentityId, name: String },
    AddCredential { identity: IdentityId },
    RevokeCredential { identity: IdentityId, credential: CredentialId },
    
    // Capability operations
    DelegateCapability { capability: CapabilityId, to: IdentityId },
    RevokeCapability { capability: CapabilityId },
    
    // System operations
    UpgradeSystem { image: ContentHash },
    ModifyPolicy { rule: PolicyRule },
}
```

---

## 3. Policy Decision

### 3.1 Decision Structure

```rust
/// Result of policy evaluation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// The decision effect
    pub effect: PolicyEffect,
    
    /// Which rules matched
    pub matched_rules: Vec<RuleId>,
    
    /// Final rule that determined the decision
    pub deciding_rule: RuleId,
    
    /// Any conditions attached to an Allow
    pub conditions: Vec<PolicyCondition>,
    
    /// Decision timestamp (Axiom time)
    pub timestamp: AxiomTime,
    
    /// Policy Engine signature
    pub signature: Signature,
    
    /// Reference to Axiom entry recording this decision
    pub axiom_ref: Option<AxiomRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyEffect {
    /// Request is allowed
    Allow,
    
    /// Request is denied
    Deny { reason: String },
    
    /// Request is allowed with conditions
    AllowWithConditions,
}
```

### 3.2 Decision Properties

| Property | Description |
|----------|-------------|
| **Deterministic** | Same policy state + same request → same decision |
| **Recorded** | Every decision is logged in the Axiom |
| **Signed** | Policy Engine signs all decisions |
| **Verifiable** | Third parties can verify decisions were correct |

---

## 4. Policy Rules

### 4.1 Rule Structure

```rust
/// A policy rule
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Unique rule identifier
    pub id: RuleId,
    
    /// Human-readable name
    pub name: String,
    
    /// Description
    pub description: String,
    
    /// Priority (higher = evaluated first)
    pub priority: u32,
    
    /// When this rule applies
    pub condition: PolicyCondition,
    
    /// What effect this rule has
    pub effect: PolicyEffect,
    
    /// Any restrictions to apply
    pub restrictions: Vec<Restriction>,
    
    /// When this rule was created
    pub created_at: AxiomRef,
    
    /// Who created this rule
    pub created_by: IdentityId,
}
```

### 4.2 Policy Conditions

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Always matches
    Always,
    
    /// Never matches
    Never,
    
    /// Match specific identity or group
    Identity(IdentityMatcher),
    
    /// Match specific resource pattern
    Resource(ResourceMatcher),
    
    /// Match specific action type
    Action(ActionMatcher),
    
    /// Combine conditions
    And(Vec<PolicyCondition>),
    Or(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
    
    /// Time-based (deterministic: Axiom time, not wall clock)
    Before(AxiomTime),
    After(AxiomTime),
    
    /// Rate limiting (count within window)
    RateLimit { count: u32, window: Duration },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IdentityMatcher {
    /// Exact identity match
    Exact(IdentityId),
    
    /// Any identity in group
    Group(GroupId),
    
    /// Any identity matching pattern
    Pattern(String),
    
    /// Any identity of type
    Type(IdentityType),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResourceMatcher {
    /// Exact resource match
    Exact(ResourceId),
    
    /// Path prefix match
    PathPrefix(PathBuf),
    
    /// Pattern match (glob-like)
    Pattern(String),
    
    /// Any resource of type
    Type(ResourceType),
}
```

---

## 5. Policy Evaluation

### 5.1 Evaluation Algorithm

```rust
impl PolicyEngine {
    /// Evaluate a policy request
    pub fn evaluate(&self, request: &PolicyRequest) -> PolicyDecision {
        // 1. Get current policy state (derived from Axiom)
        let policy_state = self.get_policy_state();
        
        // 2. Find all matching rules
        let matching_rules: Vec<&PolicyRule> = policy_state
            .rules
            .iter()
            .filter(|r| self.matches(&r.condition, request))
            .collect();
        
        // 3. Sort by priority (highest first)
        let sorted_rules = matching_rules
            .into_iter()
            .sorted_by_key(|r| Reverse(r.priority))
            .collect::<Vec<_>>();
        
        // 4. Apply first matching rule
        for rule in sorted_rules {
            match &rule.effect {
                PolicyEffect::Allow => {
                    return PolicyDecision {
                        effect: PolicyEffect::Allow,
                        matched_rules: vec![rule.id],
                        deciding_rule: rule.id,
                        conditions: vec![],
                        timestamp: self.axiom_time(),
                        signature: self.sign_decision(&request, &rule.effect),
                        axiom_ref: None, // Set after recording
                    };
                }
                PolicyEffect::Deny { reason } => {
                    return PolicyDecision {
                        effect: PolicyEffect::Deny { reason: reason.clone() },
                        matched_rules: vec![rule.id],
                        deciding_rule: rule.id,
                        conditions: vec![],
                        timestamp: self.axiom_time(),
                        signature: self.sign_decision(&request, &rule.effect),
                        axiom_ref: None,
                    };
                }
                PolicyEffect::AllowWithConditions => {
                    return PolicyDecision {
                        effect: PolicyEffect::AllowWithConditions,
                        matched_rules: vec![rule.id],
                        deciding_rule: rule.id,
                        conditions: rule.restrictions.clone(),
                        timestamp: self.axiom_time(),
                        signature: self.sign_decision(&request, &rule.effect),
                        axiom_ref: None,
                    };
                }
            }
        }
        
        // 5. Default deny if no rules match
        PolicyDecision {
            effect: PolicyEffect::Deny { 
                reason: "No matching policy rule".to_string() 
            },
            matched_rules: vec![],
            deciding_rule: RuleId::DEFAULT_DENY,
            conditions: vec![],
            timestamp: self.axiom_time(),
            signature: self.sign_decision(&request, &PolicyEffect::Deny {
                reason: "Default deny".to_string()
            }),
            axiom_ref: None,
        }
    }
}
```

### 5.2 Condition Matching

```rust
impl PolicyEngine {
    fn matches(&self, condition: &PolicyCondition, request: &PolicyRequest) -> bool {
        match condition {
            PolicyCondition::Always => true,
            PolicyCondition::Never => false,
            
            PolicyCondition::Identity(matcher) => {
                self.matches_identity(matcher, &request.requestor)
            }
            
            PolicyCondition::Resource(matcher) => {
                self.matches_resource(matcher, &request.resource)
            }
            
            PolicyCondition::Action(matcher) => {
                self.matches_action(matcher, &request.action)
            }
            
            PolicyCondition::And(conditions) => {
                conditions.iter().all(|c| self.matches(c, request))
            }
            
            PolicyCondition::Or(conditions) => {
                conditions.iter().any(|c| self.matches(c, request))
            }
            
            PolicyCondition::Not(condition) => {
                !self.matches(condition, request)
            }
            
            PolicyCondition::Before(time) => {
                self.axiom_time() < *time
            }
            
            PolicyCondition::After(time) => {
                self.axiom_time() >= *time
            }
            
            PolicyCondition::RateLimit { count, window } => {
                self.check_rate_limit(&request.requestor, *count, *window)
            }
        }
    }
}
```

---

## 6. Policy State

### 6.1 State Derivation

Policy state is derived deterministically from the Axiom:

```rust
impl PolicyState {
    /// Derive policy state from Axiom
    pub fn from_axiom(axiom: &Axiom) -> Self {
        let mut state = PolicyState::initial();
        
        for entry in axiom.entries() {
            match &entry.payload {
                EntryPayload::PolicyRuleAdded { rule } => {
                    state.rules.insert(rule.id, rule.clone());
                }
                EntryPayload::PolicyRuleRemoved { rule_id } => {
                    state.rules.remove(rule_id);
                }
                EntryPayload::CapabilityGrant { grant } => {
                    state.capabilities.insert(grant.capability_id, grant.clone());
                }
                EntryPayload::CapabilityRevoke { capability_id } => {
                    state.capabilities.remove(capability_id);
                }
                // ... other policy-affecting entries
                _ => {}
            }
        }
        
        state
    }
}
```

### 6.2 State Structure

```rust
/// Current policy state (derived from Axiom)
#[derive(Clone, Debug)]
pub struct PolicyState {
    /// Active policy rules
    pub rules: BTreeMap<RuleId, PolicyRule>,
    
    /// Active capabilities
    pub capabilities: BTreeMap<CapabilityId, CapabilityGrant>,
    
    /// Identity registry
    pub identities: BTreeMap<IdentityId, Identity>,
    
    /// Resource limits
    pub limits: BTreeMap<IdentityId, ResourceLimits>,
    
    /// Rate limit counters (in memory, reset on restart)
    pub rate_limits: BTreeMap<(IdentityId, String), RateLimitCounter>,
}
```

---

## 7. Recording Decisions

### 7.1 Decision Recording

All policy decisions are recorded in the Axiom:

```rust
impl PolicyEngine {
    /// Record a policy decision in the Axiom
    async fn record_decision(
        &self,
        request: &PolicyRequest,
        decision: &PolicyDecision,
    ) -> Result<AxiomRef, PolicyError> {
        let payload = PolicyDecisionPayload {
            request_hash: hash(request),
            requestor: request.requestor.id,
            action: request.action.clone(),
            resource: request.resource.clone(),
            policy_state_hash: self.policy_state_hash(),
            matched_rules: decision.matched_rules.clone(),
            decision: decision.effect.clone(),
            deciding_rule: decision.deciding_rule,
            conditions: decision.conditions.clone(),
            signature: decision.signature.clone(),
        };
        
        let entry = self.axiom_client
            .submit(EntryType::PolicyDecision, payload)
            .await?;
        
        Ok(entry.axiom_ref)
    }
}
```

### 7.2 Decision Verification

Third parties can verify policy decisions:

```rust
/// Verify a policy decision was correct
pub fn verify_decision(
    axiom: &Axiom,
    decision_ref: AxiomRef,
) -> Result<VerificationResult, VerifyError> {
    // 1. Get the decision entry
    let entry = axiom.get(decision_ref)?;
    let decision_payload: PolicyDecisionPayload = entry.payload.decode()?;
    
    // 2. Derive policy state at that point
    let policy_state = PolicyState::from_axiom_up_to(axiom, decision_ref)?;
    
    // 3. Re-evaluate the request
    let request = reconstruct_request(&decision_payload)?;
    let expected_decision = evaluate_with_state(&policy_state, &request);
    
    // 4. Compare
    if expected_decision.effect == decision_payload.decision 
       && expected_decision.deciding_rule == decision_payload.deciding_rule {
        Ok(VerificationResult::Verified)
    } else {
        Ok(VerificationResult::Mismatch {
            recorded: decision_payload.decision,
            expected: expected_decision.effect,
        })
    }
}
```

---

## 8. Default Policies

### 8.1 System Bootstrap Rules

```rust
/// Initial policy rules (applied at genesis)
fn bootstrap_rules() -> Vec<PolicyRule> {
    vec![
        // Allow system services to perform any action
        PolicyRule {
            id: RuleId::new("system-admin"),
            name: "System Administration".to_string(),
            priority: 1000,
            condition: PolicyCondition::Identity(
                IdentityMatcher::Type(IdentityType::System)
            ),
            effect: PolicyEffect::Allow,
            ..Default::default()
        },
        
        // Deny all by default (lowest priority)
        PolicyRule {
            id: RuleId::new("default-deny"),
            name: "Default Deny".to_string(),
            priority: 0,
            condition: PolicyCondition::Always,
            effect: PolicyEffect::Deny { 
                reason: "No explicit permission".to_string() 
            },
            ..Default::default()
        },
    ]
}
```

---

## 9. Implementation Notes

### 9.1 Performance Considerations

| Aspect | Target |
|--------|--------|
| Evaluation latency | < 100μs for simple rules |
| Rule lookup | O(log n) with rule indexing |
| State derivation | Incremental from last snapshot |

### 9.2 Caching

The Policy Engine may cache:
- Derived policy state (invalidated on new Axiom entries)
- Recent decision results (for identical requests)
- Rule indexes (for fast lookup)

Caches must be deterministic — same input always produces same output.

### 9.3 Recursion Prevention

The Policy Engine itself needs authorization for some operations (like adding rules). To prevent infinite recursion:

- The Policy Engine has bootstrap capabilities that don't require policy check
- These capabilities are minimal and cannot be expanded
- All other operations go through normal policy evaluation

---

*[← Axiom](01-axiom.md) | [Key Derivation Service →](03-keys.md)*
