# Desktop Hooks

Application-level hooks for business logic, state management, and service communication.

## Location Convention

Zero OS has **two hook directories** with distinct purposes:

### `desktop/hooks/` (this directory)

**Purpose:** Business logic, state management, and service communication

- Identity management (`useIdentity`, `useNeuralKey`, `useMachineKeys`)
- Service clients (`useIdentityServiceClient`, `useZeroIdAuth`)
- Desktop/workspace state (`useDesktops`, `useWindows`)
- System permissions (`usePermissions`)
- Context providers (`useSupervisor`, `useDesktopController`)

### `components/Desktop/hooks/`

**Purpose:** UI rendering, DOM interaction, and animation

- Render loop synchronization (`useRenderLoop`)
- Pointer/input handling (`usePointerEvents`)
- Animation frame callbacks

## Usage

Import from the index file for convenient access:

```ts
import {
  useSupervisor,
  useIdentity,
  useMachineKeys,
  useNeuralKey,
  useLinkedAccounts,
} from '../desktop/hooks';
```

Or import specific hooks directly:

```ts
import { useMachineKeys } from '../desktop/hooks/useMachineKeys';
```

## Shared Types

Hooks re-export types for backward compatibility, but canonical types are in `shared/types/`:

```ts
// Preferred: Import from shared types
import type { MachineKeyRecord, NeuralShard } from '../shared/types';

// Also works: Import from hook (re-exported)
import type { MachineKeyRecord, NeuralShard } from '../desktop/hooks';
```

## Shared Converters

Type conversion functions (snake_case ↔ camelCase) are in `shared/converters/`:

```ts
import {
  convertMachineRecord,
  convertCredential,
  convertNeuralKeyGenerated,
} from '../shared/converters/identity';
```

## Adding New Hooks

1. **Decide the location** based on purpose:
   - Business logic / service communication → `desktop/hooks/`
   - UI rendering / DOM / animation → `components/Desktop/hooks/`

2. **Add to index.ts** if placing in `desktop/hooks/`:
   ```ts
   export { useMyNewHook } from './useMyNewHook';
   export type { MyHookReturn } from './useMyNewHook';
   ```

3. **Add tests** in `__tests__/` directory
