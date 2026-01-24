# Frontend and Desktop Codebase Refactor Plan

This plan addresses violations of the React component rules (`.cursor/rules-react-components.md`) and TypeScript rules (`.cursor/rules-typescript.md`) found throughout the `web/` directory. Each phase can be completed independently and tested before moving to the next.

## Status

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Component Structure Normalization | **Completed** |
| 2 | Architecture Separation | **Completed** |
| 3 | State Management Consistency | **Completed** |
| 4 | TypeScript Strictness | **Completed** |
| 5 | Props and Component Design | **Completed** |
| 6 | Code Simplification | **Completed** |
| 7 | Accessibility and UX | **Completed** |
| 8 | Testing Infrastructure | **In Progress** |

---

## Phase 1: Component Structure Normalization

**Goal**: Ensure every component follows the folder structure rules.

### 1.1 Split Multi-Component Files

Files with multiple exported components violate "One component per file":

- `web/apps/SettingsApp/panels/MachineKeysPanel.tsx` - Extract `GenerateMachineKeyPanel` to its own folder
- `web/components/IdentityPanel/IdentityPanel.tsx` - Check for nested panel exports and extract

### 1.2 Create Proper Component Folders

Convert flat app files to folder structure with `index.ts` re-exports:

```
web/apps/SettingsApp/panels/MachineKeysPanel/
  MachineKeysPanel.tsx
  MachineKeysPanel.module.css  (extract from panels.module.css)
  MachineKeysPanel.test.tsx
  index.ts
```

Apply to all panels:

- `GeneralPanel`, `IdentitySettingsPanel`, `PermissionsPanel`, `ThemePanel`, `NetworkPanel`, `LinkedAccountsPanel`, `NeuralKeyPanel`, `ProcessCapabilitiesPanel`

### 1.3 Split Monolithic CSS

`web/apps/SettingsApp/panels/panels.module.css` (726 lines) contains styles for all panels. Extract component-specific CSS:

- Shared utility styles remain in `panels.module.css` (under 100 lines)
- Each panel gets its own CSS module with only its specific styles

**Test**: Verify each panel renders identically before/after CSS extraction.

---

## Phase 2: Architecture Separation ✅

**Goal**: Separate data orchestration from presentation per architecture rules.

### 2.1 Remove Global Module State ✅

`SettingsApp.tsx` used module-level mutable state:

```typescript
let pendingNavigation: SettingsArea | null = null;  // REMOVED
```

**Solution**: Added `pendingNavigation` state to `settingsStore.ts` with `setPendingNavigation` and `clearPendingNavigation` actions.

### 2.2 Replace Window Events with Proper State ✅

Custom events (`settings:navigate`, `paneldrill:back`) bypassed React's data flow.

**Solution**:
- Navigation state now managed via Zustand store
- Created `PanelDrillContext` with `usePanelDrill` hook for drill navigation
- `IdentityPanel` uses store's `setPendingNavigation` instead of window events

### 2.3 Create Container/Presenter Pattern ✅

Split `MachineKeysPanel` into:

```typescript
// MachineKeysPanel.tsx - Container/Data orchestration
export function MachineKeysPanel({ onDrillDown }: MachineKeysPanelProps) {
  const { state, revokeMachineKey, rotateMachineKey } = useMachineKeys();
  return <MachineKeysPanelView {...} />;
}

// MachineKeysPanelView.tsx - Pure presentation
export function MachineKeysPanelView({ machines, isLoading, onRevoke, ... }) {
  // Pure render, no hooks that touch global state
}
```

**Files Changed**:
- `web/stores/settingsStore.ts` - Added navigation state
- `web/apps/SettingsApp/context/PanelDrillContext.tsx` - New context
- `web/apps/SettingsApp/panels/MachineKeysPanel/MachineKeysPanelView.tsx` - New presenter
- Updated `IdentitySettingsPanel`, `PermissionsPanel`, `GenerateMachineKeyPanel` to use context

---

## Phase 3: State Management Consistency ✅

**Goal**: Unify state management patterns across hooks.

### 3.1 Consolidate Hook Patterns ✅

Evaluated `useLinkedAccounts` vs `useMachineKeys`:
- `useLinkedAccounts` only used in 2 related panels (IdentitySettingsPanel, LinkedAccountsPanel)
- Zustand store not needed - local state is appropriate for this use case

### 3.2 Extract Shared Utilities ✅

**User ID conversion** - Created shared utility:

```typescript
// web/services/identityUtils.ts
export function userIdToBigInt(userId: string | number | bigint | null | undefined): bigint | null
```

Exported from `web/services/index.ts`.

### 3.3 Centralize IPC Client Initialization ✅

Created shared hook:

```typescript
// web/desktop/hooks/useIdentityServiceClient.ts
export function useIdentityServiceClient(): UseIdentityServiceClientReturn
```

Provides:
- `client` - The IdentityServiceClient instance
- `userId` - Current user ID as BigInt
- `isReady` - Whether client and user are available
- `getClientOrThrow()` - Throws if client unavailable
- `getUserIdOrThrow()` - Throws if no user logged in

**Files Changed**:
- `web/services/identityUtils.ts` - New shared utility
- `web/desktop/hooks/useIdentityServiceClient.ts` - New shared hook
- `web/desktop/hooks/useLinkedAccounts.ts` - Uses shared hook
- `web/desktop/hooks/useMachineKeys.ts` - Uses shared hook
- `web/desktop/hooks/useNeuralKey.ts` - Uses shared hook

---

## Phase 4: TypeScript Strictness ✅

**Goal**: Eliminate `any` types and improve type safety.

### 4.1 Remove `any` Types ✅

Replaced `any` with `unknown` in pending request maps:

- `web/services/IdentityServiceClient.ts` - Changed `PendingRequest<any>` to `PendingRequest<unknown>`
- `web/services/TimeServiceClient.ts` - Same change

Removed 4 eslint-disable comments.

### 4.2 Add Explicit Return Types ✅

Added explicit return types to hooks missing them:

- `web/desktop/hooks/useDesktops.ts`:
  - Added `UseDesktopActionsReturn` interface
  - Added `UseVoidActionsReturn` interface
- `web/desktop/hooks/useWindows.ts`:
  - Added `UseWindowActionsReturn` interface
- `web/desktop/hooks/useKeyboardShortcuts.ts`:
  - Added `: void` return type

Service client methods already had explicit return types.

### 4.3 Replace String Unions Where Appropriate ✅

Added type-safe menu ID types:

- `MachineKeysPanel` - Already had `MachineAction` type ✅
- `IdentitySettingsPanel` - Added `IdentitySettingsMenuId` type
- `LinkedAccountsPanel` - Added `LinkedAccountMenuId` type

**Result**: TypeScript compilation passes with strict mode.

---

## Phase 5: Props and Component Design ✅

**Goal**: Improve component API design following rules.

### 5.1 Replace Prop Drilling with Context ✅

Created `PanelDrillContext` for drill-down navigation:

```typescript
// PanelDrillProvider wraps content in SettingsApp
<PanelDrillProvider onNavigateBack={navigateBack} onPushPanel={pushPanel}>
  <PanelDrill ... />
</PanelDrillProvider>

// Panels use context internally, with prop fallback for compatibility
const panelDrill = usePanelDrillOptional();
if (panelDrill) {
  panelDrill.pushPanel(item);
} else if (onDrillDown) {
  onDrillDown(item);
}
```

**Files Changed**:
- `web/apps/SettingsApp/context/PanelDrillContext.tsx` - Context implementation
- All panel components use `usePanelDrillOptional()` for navigation

### 5.2 Use Discriminated Unions for Complex State ✅

Completed in Phase 2. `MachineKeysPanelView` uses:

```typescript
export type ConfirmationState = 
  | { type: 'none' }
  | { type: 'delete'; machineId: string }
  | { type: 'rotate'; machineId: string };
```

### 5.3 Memoize Inline Callbacks ✅

Container components (`MachineKeysPanel`, `IdentitySettingsPanel`) use `useCallback` for all handlers:

```typescript
const handleConfirmDelete = useCallback(async (machineId: string) => {
  // ...
}, [revokeMachineKey]);

const handleMachineAction = useCallback((machineId: string, action: MachineAction) => {
  // ...
}, []);
```

View components have minimal inline callbacks for prop wiring, which is acceptable for presentation components.

---

## Phase 6: Code Simplification

**Goal**: Reduce complexity per "prefer simple, readable code".

### 6.1 Simplify Calculator Logic ✅

Split the 70-line `handleButton` function into focused pure functions:

```typescript
// Pure handlers that return partial state updates
function handleDigit(digit: string, state: CalcInternalState): CalcUpdate
function handleDecimal(state: CalcInternalState): CalcUpdate
function handleOperator(op: string, state: CalcInternalState): CalcUpdate
function handleClear(): CalcUpdate
function handleClearEntry(): CalcUpdate
function handleBackspace(state: CalcInternalState): CalcUpdate
function handleNegate(state: CalcInternalState): CalcUpdate
```

The main `handleButton` callback now routes to the appropriate handler and applies updates. Each handler is testable in isolation.

### 6.2 Extract Desktop.tsx Sub-Components ✅

`web/components/Desktop/Desktop.tsx` reduced from 1015 to 948 lines.

**Extracted**:
- `DesktopContextMenu.tsx` - Theme, background, and accent color menu (~120 lines)
  - Takes props for current state and callbacks
  - Self-contained with its own label mappings
  - Uses `useCallback` for menu change handling

**Kept in Desktop.tsx**:
- `DesktopInner` - Render loop with window/background coordination (deeply coupled to frame data)
- `DesktopWithPermissions` - Main orchestrator with event handling
- Window management and permission dialogs

> **Note**: Further extraction of background rendering would require refactoring the unified render loop architecture where window positions and background are updated in the same RAF callback for performance.

### 6.3 Remove Dead Code ✅

**Removed**:
- `web/desktop/types.ts` (265 lines) - Entire file was dead code
  - Contained duplicate type definitions already in `web/stores/types.ts`
  - Utility functions (`vec2Add`, `vec2Sub`, `lerpCamera`, etc.) were never imported
  - No imports from this file anywhere in the codebase

**Verified in use**:
- All exports in `web/stores/index.ts` are imported and used
- Machine keys selectors (`selectMachineKeysState`, etc.) used by `useMachineKeys` hook

---

## Phase 7: Accessibility and UX ✅

**Goal**: Meet accessibility baselines from rules.

### 7.1 Semantic HTML for Menus ✅

Converted `ContextMenu` interactive elements from `<div onClick>` to proper `<button>` elements:

**Before**:
```tsx
<div onClick={() => handleItemClick(item)} className={styles.menuItem}>
  {item.label}
</div>
```

**After**:
```tsx
<button
  type="button"
  role="menuitem"
  onClick={() => handleItemClick(item)}
  aria-checked={item.checked}
>
  {item.label}
</button>
```

**Files Changed**:
- `web/components/ContextMenu/ContextMenu.tsx` - Converted all menu items to buttons with proper ARIA roles
- `web/components/ContextMenu/ContextMenu.module.css` - Added `font-family: inherit` and `:focus-visible` styles

### 7.2 Modal Dialog Accessibility ✅

Added proper dialog semantics and focus trap to `PermissionDialog`:

- Added `role="dialog"` and `aria-modal="true"`
- Added `aria-labelledby` and `aria-describedby` for screen readers
- Implemented focus trap (Tab/Shift+Tab cycle within dialog)
- Added Escape key to close dialog
- Dialog receives focus on open

**Files Changed**:
- `web/components/PermissionDialog/PermissionDialog.tsx`

### 7.3 Icon Button Labels ✅

Added `aria-label` attributes to all icon-only buttons in the Taskbar:

- Begin Menu button: `aria-label`, `aria-expanded`, `aria-haspopup="menu"`
- Desktop buttons: `aria-label`, `aria-pressed`
- Add Desktop button: `aria-label`
- Wallet button: `aria-label`
- Neural Link button: `aria-label`, `aria-expanded`, `aria-haspopup`

**Files Changed**:
- `web/components/Taskbar/Taskbar.tsx`

### 7.4 Window Resize Handle Labels ✅

Added accessibility attributes to window resize handles:

- Edge handles: `role="separator"`, `aria-orientation`, `aria-label`, `aria-valuenow`
- Corner handles: `aria-label`

**Files Changed**:
- `web/components/WindowContent/WindowContent.tsx`

**Remaining for Future**:
- Full keyboard navigation for Begin Menu (currently uses zui Menu component)
- Settings panel keyboard navigation (handled by zui PanelDrill)
- aXe accessibility audit for comprehensive coverage

---

## Phase 8: Testing Infrastructure (In Progress)

**Goal**: Add missing test files and improve testability.

### 8.1 Add Component Tests

Create test files for components missing them:

- ✅ `web/apps/CalculatorApp/CalculatorApp.test.tsx` - 26 tests covering:
  - Rendering (display, digit buttons, operator buttons, utility buttons)
  - Digit input (single digits, multi-digit, replacing 0)
  - Decimal input (adding point, leading zero, ignoring second decimal)
  - Basic operations (add, subtract, multiply, divide, division by zero)
  - Chained operations
  - Clear functions (C, CE, clearing error state)
  - Backspace functionality
  - Negate functionality
  - State after computation
  - External message handler registration
- `web/apps/SettingsApp/SettingsApp.test.tsx` - Pending
- `web/apps/TerminalApp/TerminalApp.test.tsx` - Pending
- Panel component tests - Pending

### 8.2 Hook Testing

Add tests for hooks in `web/desktop/hooks/`:

- ✅ `useIdentityServiceClient.test.ts` - 12 tests covering:
  - Initialization (null client without supervisor, throws when unavailable)
  - userId conversion (BigInt when logged in, null when not)
  - isReady state (false without supervisor, false without user)
  - getClientOrThrow (throws when unavailable, returns client after effect)
  - getUserIdOrThrow (throws when no user, returns userId)
  - Callback stability (getClientOrThrow stable, getUserIdOrThrow changes with userId)
- `useMachineKeys.test.ts` - Pending
- `useLinkedAccounts.test.ts` - Pending
- `useNeuralKey.test.ts` - Pending

### 8.3 Service Client Testing

Expand `web/services/__tests__/IdentityServiceClient.test.ts` coverage - Pending.

**Test**: Target 80% coverage on refactored code.

**Files Created**:
- `web/apps/CalculatorApp/CalculatorApp.test.tsx`
- `web/desktop/hooks/__tests__/useIdentityServiceClient.test.ts`

---

## Summary: Priority Order

| Phase | Effort | Impact | Risk |
|-------|--------|--------|------|
| 1: Component Structure | Medium | High | Low |
| 2: Architecture | High | High | Medium |
| 3: State Management | Medium | Medium | Low |
| 4: TypeScript | Low | Medium | Low |
| 5: Props Design | Medium | Medium | Low |
| 6: Simplification | Medium | High | Low |
| 7: Accessibility | Low | Medium | Low |
| 8: Testing | Medium | High | Low |

**Recommended Start**: Phase 1 (low risk, establishes foundation) followed by Phase 4 (quick wins with type safety).
