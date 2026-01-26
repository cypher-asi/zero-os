# Shared Utilities

Common types, converters, and utilities used across the Zero OS frontend.

## Directory Structure

```
shared/
  types/              # Canonical type definitions
    supervisor.ts     # Supervisor interface (single source of truth)
    identity.ts       # UI types for identity (camelCase)
    index.ts
  
  converters/         # Type conversion functions
    identity.ts       # Service ↔ UI type converters
    index.ts
  
  ipc/                # IPC utilities
    PendingRequestQueue.ts  # Shared request queue
    index.ts
  
  index.ts            # Re-exports everything
```

## Types (`shared/types/`)

### Supervisor

The `Supervisor` interface is the bridge to the zos-supervisor WASM module.
All components should import from here:

```ts
import type { Supervisor, MinimalSupervisor } from '../shared/types';
```

- `Supervisor` - Full interface with all kernel APIs
- `MinimalSupervisor` - Subset for service clients (IPC only)

### Identity Types

UI types use **camelCase** naming convention:

```ts
import type {
  MachineKeyRecord,      // machineId, signingPublicKey, ...
  NeuralShard,           // index, hex
  LinkedCredential,      // type, identifier, verified, ...
} from '../shared/types';
```

Service layer types (snake_case) are in `client-services/identity/types.ts`.

## Converters (`shared/converters/`)

Convert between service types (snake_case) and UI types (camelCase):

```ts
import {
  convertMachineRecord,      // Service → UI
  convertCapabilitiesForService,  // UI → Service
  convertCredential,
  convertNeuralKeyGenerated,
} from '../shared/converters';
```

### When to Use

- **Reading from service:** Convert service response to UI type
- **Writing to service:** Convert UI type to service format
- **Displaying in React:** Use UI types (camelCase)

## IPC Utilities (`shared/ipc/`)

### PendingRequestQueue

Manages pending IPC requests with FIFO queues per response tag:

```ts
import { PendingRequestQueue } from '../shared/ipc';

const queue = new PendingRequestQueue({ name: 'MyServiceClient' });
queue.register(supervisor);

// In request method:
const tagHex = supervisor.send_service_ipc('myservice', tag, data);
return queue.addRequest<ResponseType>(tagHex, timeoutMs);
```

Features:
- Handles concurrent requests of the same message type
- FIFO resolution order
- Timeout handling with unique request IDs

## Usage

Import from the root index:

```ts
// All shared utilities
import { Supervisor, MachineKeyRecord, convertMachineRecord } from '../shared';

// Or specific modules
import type { Supervisor } from '../shared/types';
import { convertMachineRecord } from '../shared/converters';
import { PendingRequestQueue } from '../shared/ipc';
```
