# Zero OS Applications

This directory contains the React applications that run within Zero OS windows.

## App Structure Patterns

### Simple Apps (1-3 files)

For simple applications with minimal state and few components:

```
AppName/
  AppName.tsx           # Main component
  AppName.module.css    # Styles
  AppName.test.tsx      # Tests (optional)
```

**Examples:** `CalculatorApp/`, `ClockApp/`, `TerminalApp/`

### Complex Apps (many files)

For complex applications with multiple views, shared state, or many components:

```
AppName/
  AppName.tsx           # Main component / layout
  AppName.module.css    # Root styles
  AppName.test.tsx      # Integration tests
  index.ts              # Public exports
  
  components/           # Shared UI components
    MyComponent/
      MyComponent.tsx
      MyComponent.module.css
      index.ts
  
  panels/               # Sub-views / screens
    PanelName/
      PanelName.tsx
      PanelName.module.css
      index.ts
  
  context/              # React context providers
    MyContext.tsx
    index.ts
```

**Example:** `SettingsApp/`

## Wire Format Protocol

The `_wire-format/` directory contains TypeScript types for IPC communication with backend services:

```
_wire-format/
  app-protocol.ts       # Main protocol definitions
  protocol/
    calculator.ts       # Calculator-specific messages
    settings.ts         # Settings-specific messages
    types.ts            # Shared types
    index.ts
```

## Adding a New App

1. **Create the directory structure** based on complexity (simple or complex)

2. **Create the main component:**
   ```tsx
   // MyApp/MyApp.tsx
   import styles from './MyApp.module.css';
   
   export function MyApp() {
     return <div className={styles.container}>...</div>;
   }
   ```

3. **Create CSS module:**
   ```css
   /* MyApp/MyApp.module.css */
   .container {
     /* styles */
   }
   ```

4. **Register in AppRouter** (`AppRouter/AppRouter.tsx`):
   ```tsx
   case 'myapp':
     return <MyApp />;
   ```

5. **Add app manifest** if needed for permissions

## Style Guidelines

- Use CSS Modules for component-scoped styles
- Shared panel styles can go in `panels/panels.module.css`
- Follow existing naming conventions (PascalCase for components, camelCase for CSS classes)

## Testing

- Place tests next to the component: `MyApp.test.tsx`
- Integration tests can test the full app with mocked services
- Use the test utilities from `test/mocks/`
