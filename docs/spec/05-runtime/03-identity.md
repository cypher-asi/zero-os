# Identity Service

> User and service identity management.

## Overview

The Identity Service provides:

1. **User Authentication**: Verify user credentials
2. **Service Identity**: Issue and verify service tokens
3. **Key Management**: Secure storage of cryptographic keys
4. **Session Management**: Track active user sessions

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Identity Service                              │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Identity Store                               ││
│  │                                                                ││
│  │  Users:                                                        ││
│  │  • alice: { hash: ..., keys: [...], roles: [admin] }          ││
│  │  • bob: { hash: ..., keys: [...], roles: [user] }             ││
│  │                                                                ││
│  │  Services:                                                     ││
│  │  • storage: { public_key: ..., roles: [system] }              ││
│  │  • network: { public_key: ..., roles: [system] }              ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Session Store                                ││
│  │                                                                ││
│  │  Token          │ User  │ Created    │ Expires                 ││
│  │  ────────────────┼───────┼────────────┼─────────               ││
│  │  abc123...      │ alice │ 1234567890 │ 1234657890              ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Message Handlers:                                                   │
│  • AUTHENTICATE   → verify credentials, issue token                 │
│  • VERIFY_TOKEN   → check if token is valid                         │
│  • CREATE_USER    → register new user (admin)                       │
│  • REGISTER_SVC   → register service identity                       │
│  • SIGN_DATA      → sign data with service key                      │
└─────────────────────────────────────────────────────────────────────┘
```

## IPC Protocol

### Authentication

```rust
/// Authentication request.
pub const MSG_AUTH: u32 = 0x6000;
/// Authentication response.
pub const MSG_AUTH_RESPONSE: u32 = 0x6001;

/// Authentication request.
#[derive(Clone, Debug)]
pub struct AuthRequest {
    /// Username
    pub username: String,
    /// Authentication method
    pub method: AuthMethod,
}

#[derive(Clone, Debug)]
pub enum AuthMethod {
    /// Password-based authentication
    Password { password: String },
    /// Public key challenge-response
    PublicKey { signature: Vec<u8>, challenge: Vec<u8> },
    /// Token refresh
    Token { refresh_token: String },
}

/// Authentication response.
#[derive(Clone, Debug)]
pub struct AuthResponse {
    pub result: Result<AuthSuccess, AuthError>,
}

pub struct AuthSuccess {
    /// Session token
    pub token: String,
    /// Token expiry (nanos since boot)
    pub expires_at: u64,
    /// User's roles
    pub roles: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum AuthError {
    InvalidCredentials,
    UserNotFound,
    AccountLocked,
    TokenExpired,
}
```

### Token Verification

```rust
/// Verify token request.
pub const MSG_VERIFY: u32 = 0x6002;
/// Verify token response.
pub const MSG_VERIFY_RESPONSE: u32 = 0x6003;

/// Token verification request.
pub struct VerifyRequest {
    pub token: String,
}

/// Token verification response.
pub struct VerifyResponse {
    pub valid: bool,
    pub identity: Option<Identity>,
}

/// Verified identity.
pub struct Identity {
    pub id: String,
    pub identity_type: IdentityType,
    pub roles: Vec<String>,
}

pub enum IdentityType {
    User,
    Service,
    System,
}
```

### Service Registration

```rust
/// Register service identity.
pub const MSG_REGISTER_SERVICE: u32 = 0x6004;
/// Service registration response.
pub const MSG_REGISTER_RESPONSE: u32 = 0x6005;

/// Service registration.
pub struct ServiceRegistration {
    /// Service name
    pub name: String,
    /// Service's public key (for verification)
    pub public_key: Vec<u8>,
}

/// Service registration response.
pub struct ServiceRegistrationResponse {
    /// Service ID
    pub service_id: String,
    /// Service token (long-lived)
    pub token: String,
}
```

## Identity Model

```rust
/// A user identity.
#[derive(Clone, Debug)]
pub struct UserIdentity {
    /// Unique user ID
    pub id: String,
    /// Username
    pub username: String,
    /// Password hash (argon2)
    pub password_hash: Vec<u8>,
    /// Public keys for key-based auth
    pub public_keys: Vec<PublicKey>,
    /// Assigned roles
    pub roles: Vec<String>,
    /// Account status
    pub status: AccountStatus,
    /// Creation timestamp
    pub created_at: u64,
}

pub struct PublicKey {
    pub algorithm: String,  // "ed25519", "ecdsa-p256"
    pub key_data: Vec<u8>,
    pub added_at: u64,
}

pub enum AccountStatus {
    Active,
    Locked { reason: String, until: Option<u64> },
    Disabled,
}

/// A service identity.
#[derive(Clone, Debug)]
pub struct ServiceIdentity {
    pub id: String,
    pub name: String,
    pub public_key: Vec<u8>,
    pub roles: Vec<String>,
    pub created_at: u64,
}
```

## Session Management

```rust
/// Active session.
pub struct Session {
    /// Session token
    pub token: String,
    /// Identity ID
    pub identity_id: String,
    /// Identity type
    pub identity_type: IdentityType,
    /// Creation time
    pub created_at: u64,
    /// Expiry time
    pub expires_at: u64,
    /// Last activity
    pub last_active: u64,
}

impl IdentityService {
    fn create_session(&mut self, identity_id: &str, identity_type: IdentityType) -> Session {
        let token = generate_secure_token();
        let now = self.now();
        
        let session = Session {
            token: token.clone(),
            identity_id: identity_id.to_string(),
            identity_type,
            created_at: now,
            expires_at: now + SESSION_DURATION,
            last_active: now,
        };
        
        self.sessions.insert(token.clone(), session.clone());
        session
    }
    
    fn verify_session(&mut self, token: &str) -> Option<&Session> {
        let session = self.sessions.get_mut(token)?;
        
        let now = self.now();
        if session.expires_at < now {
            self.sessions.remove(token);
            return None;
        }
        
        session.last_active = now;
        self.sessions.get(token)
    }
}

fn generate_secure_token() -> String {
    let mut bytes = [0u8; 32];
    random_bytes(&mut bytes);
    hex_encode(&bytes)
}
```

## Key Operations

```rust
/// Sign data request.
pub const MSG_SIGN: u32 = 0x6006;
/// Sign response.
pub const MSG_SIGN_RESPONSE: u32 = 0x6007;

/// Signing request.
pub struct SignRequest {
    /// Service token (must be authenticated)
    pub token: String,
    /// Data to sign
    pub data: Vec<u8>,
}

/// Signing response.
pub struct SignResponse {
    pub signature: Vec<u8>,
}

impl IdentityService {
    fn sign_data(&self, service_id: &str, data: &[u8]) -> Result<Vec<u8>, SignError> {
        // Get service's private key from secure storage
        let private_key = self.get_service_private_key(service_id)?;
        
        // Sign using Ed25519
        let signature = ed25519_sign(&private_key, data);
        
        Ok(signature)
    }
    
    fn verify_signature(
        &self,
        service_id: &str,
        data: &[u8],
        signature: &[u8],
    ) -> bool {
        let service = match self.services.get(service_id) {
            Some(s) => s,
            None => return false,
        };
        
        ed25519_verify(&service.public_key, data, signature)
    }
}
```

## Roles and Authorization

Roles are used by other services for authorization:

```rust
/// Well-known roles.
pub mod roles {
    /// System administrator
    pub const ADMIN: &str = "admin";
    /// Regular user
    pub const USER: &str = "user";
    /// Guest (limited access)
    pub const GUEST: &str = "guest";
    /// System service
    pub const SYSTEM: &str = "system";
}

impl IdentityService {
    /// Check if identity has a role.
    fn has_role(&self, identity_id: &str, role: &str) -> bool {
        // Check users
        if let Some(user) = self.users.get(identity_id) {
            return user.roles.contains(&role.to_string());
        }
        
        // Check services
        if let Some(service) = self.services.get(identity_id) {
            return service.roles.contains(&role.to_string());
        }
        
        false
    }
}
```

## WASM Implementation

```rust
// identity_service.rs

#![no_std]
extern crate alloc;
extern crate orbital_process;

use alloc::collections::BTreeMap;
use orbital_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("identity: starting");
    
    // Load identity store from secure storage
    let store = load_identity_store();
    
    let service_ep = create_endpoint();
    register_service("identity", service_ep);
    send_ready();
    
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            MSG_AUTH => handle_auth(msg),
            MSG_VERIFY => handle_verify(msg),
            MSG_REGISTER_SERVICE => handle_register_service(msg),
            MSG_SIGN => handle_sign(msg),
            _ => debug("identity: unknown message"),
        }
    }
}
```

## Security Considerations

1. **Password Storage**: Use Argon2 with appropriate parameters
2. **Token Generation**: Use cryptographically secure random
3. **Key Storage**: Private keys stored in secure/encrypted storage
4. **Session Expiry**: Tokens expire and require refresh
5. **Rate Limiting**: Protect against brute force attacks

```rust
/// Password hashing parameters.
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_MEMORY_COST: u32 = 65536;
const ARGON2_PARALLELISM: u32 = 1;

fn hash_password(password: &str) -> Vec<u8> {
    let salt = generate_random_salt();
    argon2_hash(
        password.as_bytes(),
        &salt,
        ARGON2_TIME_COST,
        ARGON2_MEMORY_COST,
        ARGON2_PARALLELISM,
    )
}

fn verify_password(password: &str, hash: &[u8]) -> bool {
    argon2_verify(password.as_bytes(), hash)
}
```
