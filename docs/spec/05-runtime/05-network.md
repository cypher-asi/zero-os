# Network Service

> Network connectivity for applications.

## Overview

The Network Service provides:

1. **HTTP/Fetch**: Make HTTP requests to external servers
2. **WebSocket**: Bidirectional real-time connections (future)
3. **DNS**: Name resolution (future)
4. **Network Policy**: Control which hosts apps can access

## WASM Implementation

On WASM, networking uses the browser's Fetch API:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Network Service                               │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Fetch Backend                                ││
│  │                                                                ││
│  │  • HTTP GET/POST/PUT/DELETE                                    ││
│  │  • Headers management                                          ││
│  │  • CORS handling                                               ││
│  │  • Response streaming                                          ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Network Policy                               ││
│  │                                                                ││
│  │  Allow: *.api.example.com                                      ││
│  │  Allow: cdn.example.com                                        ││
│  │  Deny: *.malware.com                                           ││
│  │  Default: Deny                                                 ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Message Handlers:                                                   │
│  • NET_REQUEST    → make HTTP request                               │
│  • NET_WEBSOCKET  → open WebSocket connection                       │
│  • NET_POLICY     → query/update policy                             │
└─────────────────────────────────────────────────────────────────────┘
```

## IPC Protocol

### HTTP Request

```rust
/// Network request.
pub const MSG_NET_REQUEST: u32 = 0x8000;
/// Network response.
pub const MSG_NET_RESPONSE: u32 = 0x8001;

/// HTTP request.
#[derive(Clone, Debug)]
pub struct HttpRequest {
    /// HTTP method
    pub method: HttpMethod,
    /// URL to fetch
    pub url: String,
    /// Request headers
    pub headers: Vec<(String, String)>,
    /// Request body (for POST/PUT)
    pub body: Option<Vec<u8>>,
    /// Timeout in milliseconds
    pub timeout_ms: u32,
}

#[derive(Clone, Copy, Debug)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// HTTP response.
#[derive(Clone, Debug)]
pub struct HttpResponse {
    pub result: Result<HttpSuccess, NetworkError>,
}

pub struct HttpSuccess {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum NetworkError {
    /// URL not allowed by policy
    PolicyDenied,
    /// DNS resolution failed
    DnsError,
    /// Connection failed
    ConnectionFailed,
    /// Request timed out
    Timeout,
    /// Invalid URL
    InvalidUrl,
    /// CORS error
    CorsError,
    /// Other error
    Other(String),
}
```

### WebSocket (Future)

```rust
/// WebSocket open request.
pub const MSG_WS_OPEN: u32 = 0x8010;
/// WebSocket message (bidirectional).
pub const MSG_WS_MESSAGE: u32 = 0x8011;
/// WebSocket close.
pub const MSG_WS_CLOSE: u32 = 0x8012;

/// WebSocket open request.
pub struct WsOpenRequest {
    /// WebSocket URL (ws:// or wss://)
    pub url: String,
    /// Subprotocols
    pub protocols: Vec<String>,
}

/// WebSocket opened.
pub struct WsOpened {
    /// Connection ID
    pub conn_id: u64,
    /// Endpoint for receiving messages
    pub message_endpoint: CapSlot,
}

/// WebSocket message.
pub struct WsMessage {
    pub conn_id: u64,
    pub data: WsData,
}

pub enum WsData {
    Text(String),
    Binary(Vec<u8>),
}
```

## Network Policy

Policy controls which URLs applications can access:

```rust
/// Network policy rule.
#[derive(Clone, Debug)]
pub struct NetworkPolicyRule {
    /// Rule ID
    pub id: String,
    /// Process class this applies to
    pub applies_to: ProcessClass,
    /// URL pattern (glob)
    pub url_pattern: String,
    /// Whether to allow or deny
    pub allow: bool,
    /// Priority (higher = evaluated first)
    pub priority: u32,
}

impl NetworkService {
    fn check_policy(&self, caller: ProcessId, url: &str) -> Result<(), NetworkError> {
        let class = self.classify_process(caller);
        
        // Parse URL to extract host
        let host = parse_url_host(url)?;
        
        // Check rules in priority order
        let mut rules: Vec<_> = self.policy.iter()
            .filter(|r| self.rule_applies(r, &class))
            .collect();
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        
        for rule in rules {
            if glob_match(&rule.url_pattern, url) || glob_match(&rule.url_pattern, &host) {
                if rule.allow {
                    return Ok(());
                } else {
                    return Err(NetworkError::PolicyDenied);
                }
            }
        }
        
        // Default deny
        Err(NetworkError::PolicyDenied)
    }
}

/// Default network policy.
fn default_policy() -> Vec<NetworkPolicyRule> {
    vec![
        // System services can access anything
        NetworkPolicyRule {
            id: "system-allow-all".to_string(),
            applies_to: ProcessClass::System,
            url_pattern: "*".to_string(),
            allow: true,
            priority: 100,
        },
        
        // Apps can access HTTPS only
        NetworkPolicyRule {
            id: "app-https-only".to_string(),
            applies_to: ProcessClass::Application,
            url_pattern: "https://*".to_string(),
            allow: true,
            priority: 50,
        },
        
        // Block known bad domains
        NetworkPolicyRule {
            id: "block-malware".to_string(),
            applies_to: ProcessClass::Application,
            url_pattern: "*.malware.example.com".to_string(),
            allow: false,
            priority: 90,
        },
    ]
}
```

## Fetch Backend (WASM)

```javascript
// network_backend.js

class NetworkBackend {
    async fetch(request) {
        const { method, url, headers, body, timeout_ms } = request;
        
        // Create abort controller for timeout
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeout_ms);
        
        try {
            const response = await fetch(url, {
                method,
                headers: new Headers(headers),
                body: body ? new Uint8Array(body) : undefined,
                signal: controller.signal,
            });
            
            clearTimeout(timeoutId);
            
            // Read response
            const responseBody = await response.arrayBuffer();
            const responseHeaders = [];
            response.headers.forEach((value, key) => {
                responseHeaders.push([key, value]);
            });
            
            return {
                status: response.status,
                headers: responseHeaders,
                body: new Uint8Array(responseBody),
            };
        } catch (error) {
            clearTimeout(timeoutId);
            
            if (error.name === 'AbortError') {
                throw { type: 'Timeout' };
            }
            throw { type: 'ConnectionFailed', message: error.message };
        }
    }
}
```

## Native Backend (Future)

On native targets, use sockets directly:

```rust
// native_network.rs (future)

struct NativeNetwork {
    // Connection pool, DNS resolver, etc.
}

impl NativeNetwork {
    async fn http_request(&self, request: HttpRequest) -> Result<HttpSuccess, NetworkError> {
        // 1. Parse URL
        let url = Url::parse(&request.url)?;
        
        // 2. DNS resolution
        let addr = self.resolve(&url.host())?;
        
        // 3. TCP connection (with TLS for HTTPS)
        let stream = if url.scheme() == "https" {
            TlsStream::connect(addr, url.host())?
        } else {
            TcpStream::connect(addr)?
        };
        
        // 4. Send HTTP request
        let http_request = format_http_request(&request);
        stream.write_all(&http_request)?;
        
        // 5. Read response
        let response = parse_http_response(&stream)?;
        
        Ok(response)
    }
}
```

## WASM Implementation

```rust
// network_service.rs

#![no_std]
extern crate alloc;
extern crate orbital_process;

use orbital_process::*;

#[no_mangle]
pub extern "C" fn _start() {
    debug("network: starting");
    
    // Load network policy
    let policy = load_policy_or_default();
    
    let service_ep = create_endpoint();
    register_service("network", service_ep);
    send_ready();
    
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            MSG_NET_REQUEST => handle_http_request(msg),
            MSG_WS_OPEN => handle_ws_open(msg),
            MSG_WS_MESSAGE => handle_ws_message(msg),
            MSG_WS_CLOSE => handle_ws_close(msg),
            MSG_NET_POLICY => handle_policy_query(msg),
            _ => debug("network: unknown message"),
        }
    }
}

fn handle_http_request(msg: ReceivedMessage) {
    let request: HttpRequest = decode(&msg.data);
    
    // Check policy
    if let Err(e) = check_policy(msg.from, &request.url) {
        send_response(msg, HttpResponse { result: Err(e) });
        return;
    }
    
    // Make request via backend
    match backend_fetch(&request) {
        Ok(response) => send_response(msg, HttpResponse { result: Ok(response) }),
        Err(e) => send_response(msg, HttpResponse { result: Err(e) }),
    }
}
```

## Rate Limiting

Prevent abuse by rate limiting requests:

```rust
struct RateLimiter {
    /// Process -> (count, window_start)
    requests: BTreeMap<ProcessId, (u32, u64)>,
}

impl RateLimiter {
    fn check(&mut self, pid: ProcessId, now: u64) -> bool {
        let window_duration = 60_000_000_000;  // 1 minute in nanos
        let max_requests = 100;  // 100 requests per minute
        
        let entry = self.requests.entry(pid).or_insert((0, now));
        
        // Reset window if expired
        if now - entry.1 > window_duration {
            *entry = (0, now);
        }
        
        // Check limit
        if entry.0 >= max_requests {
            return false;
        }
        
        entry.0 += 1;
        true
    }
}
```
