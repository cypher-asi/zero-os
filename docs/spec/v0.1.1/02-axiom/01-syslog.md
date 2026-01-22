# SysLog - System Event Log

## Overview

SysLog records all syscalls (request + response) for auditing. This is separate from CommitLog—SysLog is for auditing, CommitLog is for deterministic replay.

## Event Structure

```rust
pub struct SysEvent {
    /// Unique event ID (monotonic)
    pub id: EventId,
    /// Process that made the syscall
    pub sender: ProcessId,
    /// Timestamp (nanos since boot)
    pub timestamp: u64,
    /// Event type (request or response)
    pub event_type: SysEventType,
}

pub enum SysEventType {
    /// Syscall request from a process
    Request {
        syscall_num: u32,
        args: [u32; 4],
    },
    /// Syscall response to a process
    Response {
        request_id: EventId,
        result: i64,
    },
}
```

## Event Lifecycle

Every syscall generates exactly two events:

1. **Request Event**: When the syscall is received
2. **Response Event**: When the syscall completes

The response links back to the request via `request_id`.

## Example Trace

```
ID  Time(ns)  Sender  Type      Details
─────────────────────────────────────────────────────────
0   1000      1       Request   SYS_EP_CREATE args=[0,0,0,0]
1   1050      1       Response  request=0 result=0
2   2000      2       Request   SYS_SEND args=[0,0x2000,12,0]
3   2100      2       Response  request=2 result=0
4   3000      1       Request   SYS_RECEIVE args=[1,0,0,0]
5   3010      1       Response  request=4 result=12
```

## API

### Logging

```rust
impl SysLog {
    /// Log a syscall request, returns event ID
    pub fn log_request(
        &mut self,
        sender: ProcessId,
        syscall_num: u32,
        args: [u32; 4],
        timestamp: u64,
    ) -> EventId;

    /// Log a syscall response
    pub fn log_response(
        &mut self,
        sender: ProcessId,
        request_id: EventId,
        result: i64,
        timestamp: u64,
    );
}
```

### Querying

```rust
impl SysLog {
    /// Get all events
    pub fn events(&self) -> &[SysEvent];

    /// Get events in a range [start_id, end_id)
    pub fn get_range(&self, start_id: EventId, end_id: EventId) -> Vec<&SysEvent>;

    /// Get most recent N events
    pub fn get_recent(&self, count: usize) -> Vec<&SysEvent>;

    /// Current event count
    pub fn len(&self) -> usize;

    /// Next event ID that will be assigned
    pub fn next_id(&self) -> EventId;
}
```

## Memory Management

SysLog trims old events when exceeding capacity:

```rust
const MAX_SYSLOG_EVENTS: usize = 10000;
```

Events are trimmed from the front (oldest) to maintain recent history.

## Use Cases

### Security Auditing

- Track all syscalls by a suspicious process
- Verify capability usage patterns
- Detect unauthorized access attempts

### Debugging

- Trace syscall sequences during development
- Correlate errors with preceding syscalls
- Profile syscall latency (response timestamp - request timestamp)

### Compliance

- Prove system behavior for regulatory requirements
- Log chain of custody for sensitive operations
- Record all permission changes

## Compliance Checklist

### Source Files
- `crates/zos-axiom/src/syslog.rs`

### Key Invariants
- [ ] Event IDs are monotonically increasing
- [ ] Every request has exactly one response
- [ ] Response links to correct request
- [ ] Timestamps are non-decreasing
- [ ] Trim preserves most recent events

### Differences from v0.1.0
- Event structure unchanged
- Added get_recent() for efficient recent history access
- Capacity limit is now configurable (was hardcoded)
