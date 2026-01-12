# Phase 4: Networking

**Duration:** 6-8 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 3 (Storage & Filesystem)

---

## Objective

Implement a user-space TCP/IP network stack with Axiom-based connection authorization.

---

## Deliverables

### 4.1 Network Driver

| Component | Description | Complexity |
|-----------|-------------|------------|
| virtio-net driver | QEMU network device | Medium |
| Packet buffer | RX/TX queues | Medium |
| MAC handling | Ethernet frames | Low |
| Interrupt handling | Packet notification | Low |

### 4.2 IP Layer

| Component | Description | Complexity |
|-----------|-------------|------------|
| IPv4 | IP packet handling | Medium |
| IPv6 | IPv6 support | Medium |
| ARP | Address resolution | Medium |
| ICMP | Ping support | Low |
| Routing table | Route lookup | Medium |

### 4.3 Transport Layer

| Component | Description | Complexity |
|-----------|-------------|------------|
| TCP | Full TCP implementation | High |
| UDP | Datagram support | Low |
| Port management | Port allocation | Low |

### 4.4 Socket Layer

| Component | Description | Complexity |
|-----------|-------------|------------|
| Socket API | BSD-like interface | Medium |
| Socket table | Connection tracking | Medium |
| Select/poll | I/O multiplexing | Medium |

### 4.5 Authorization

| Component | Description | Complexity |
|-----------|-------------|------------|
| Connection auth | Axiom integration | Medium |
| Policy enforcement | Rule matching | Medium |
| Audit logging | Connection logging | Low |

---

## Technical Approach

### Network Service

```rust
pub struct NetworkService {
    /// Network driver
    driver: NetworkDriver,
    
    /// IP layer
    ip: IpLayer,
    
    /// TCP connections
    tcp: TcpManager,
    
    /// UDP sockets
    udp: UdpManager,
    
    /// Axiom for authorization
    axiom: AxiomClient,
}

impl NetworkService {
    pub async fn connect(
        &mut self,
        socket: SocketId,
        addr: SocketAddr,
    ) -> Result<(), NetError> {
        // Request authorization from Axiom
        let proposal = Proposal {
            entry_type: EntryType::ConnectionAuthorize,
            payload: ConnectionAuthPayload {
                local_addr: self.get_local_addr(socket)?,
                remote_addr: addr,
                protocol: Protocol::Tcp,
                direction: Direction::Outbound,
            }.into(),
            ..Default::default()
        };
        
        let result = self.axiom.submit(proposal).await?;
        match result {
            CommitResult::Committed { sequence, .. } => {
                // Authorized - proceed with connection
                self.tcp.connect(socket, addr, sequence).await
            }
            CommitResult::Rejected { .. } => {
                Err(NetError::AuthorizationDenied)
            }
            _ => Err(NetError::Internal),
        }
    }
    
    pub async fn send(
        &mut self,
        socket: SocketId,
        data: &[u8],
    ) -> Result<usize, NetError> {
        // Data-plane - no Axiom involvement
        self.tcp.send(socket, data).await
    }
}
```

### TCP State Machine

```rust
pub struct TcpConnection {
    state: TcpState,
    local: SocketAddr,
    remote: SocketAddr,
    send_seq: TcpSeqState,
    recv_seq: TcpSeqState,
    send_buffer: RingBuffer,
    recv_buffer: RingBuffer,
    authorization: u64, // Axiom entry
}

impl TcpConnection {
    pub async fn process_segment(&mut self, segment: TcpSegment) -> Result<(), TcpError> {
        match self.state {
            TcpState::SynSent => {
                if segment.flags.contains(TcpFlags::SYN | TcpFlags::ACK) {
                    // Connection established
                    self.recv_seq.nxt = segment.seq.wrapping_add(1);
                    self.state = TcpState::Established;
                    
                    // Send ACK
                    self.send_ack().await?;
                }
            }
            TcpState::Established => {
                // Process data
                if !segment.payload.is_empty() {
                    self.recv_buffer.write(&segment.payload)?;
                    self.recv_seq.nxt = self.recv_seq.nxt.wrapping_add(segment.payload.len() as u32);
                    self.send_ack().await?;
                }
                
                // Check for FIN
                if segment.flags.contains(TcpFlags::FIN) {
                    self.state = TcpState::CloseWait;
                }
            }
            // ... other states
            _ => {}
        }
        
        Ok(())
    }
}
```

---

## Implementation Steps

### Week 1-2: Network Driver & IP

1. Implement virtio-net driver
2. Add packet buffer management
3. Implement ARP
4. Implement IPv4
5. Add ICMP (ping)
6. Create routing table

### Week 3-4: TCP

1. Implement TCP state machine
2. Add connection establishment
3. Implement data transfer
4. Add flow control
5. Implement connection teardown
6. Add retransmission

### Week 5-6: UDP & Sockets

1. Implement UDP
2. Create socket abstraction
3. Add socket table
4. Implement select/poll
5. Add socket options

### Week 7-8: Authorization & Testing

1. Implement Axiom authorization
2. Add policy enforcement
3. Integration testing
4. Network testing
5. Documentation

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Ping works | ICMP test |
| TCP connections work | Client/server test |
| Authorization enforced | Policy test |
| Data-plane not logged | Axiom inspection |
| Performance acceptable | Throughput test |

---

*[← Phase 3](03-phase-storage-filesystem.md) | [Phase 5: Isolation →](05-phase-isolation-tiers.md)*
