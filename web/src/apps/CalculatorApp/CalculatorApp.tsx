import { useState, useCallback } from 'react';
import { Panel, Button, Text, Label } from '@cypher-asi/zui';
import { decodeCalculatorState, CalculatorState } from '../_wire-format/app-protocol';
import styles from './CalculatorApp.module.css';

// =============================================================================
// Calculator State Types
// =============================================================================

/** Internal calculator state for computation */
interface CalcInternalState {
  display: string;
  pendingOp: string | null;
  hasError: boolean;
  accumulator: number;
  justComputed: boolean;
}

/** Result of a button handler - partial state update */
type CalcUpdate = Partial<CalcInternalState>;

// =============================================================================
// Button Handlers - Pure functions that return state updates
// =============================================================================

/** Handle digit input (0-9) */
function handleDigit(digit: string, state: CalcInternalState): CalcUpdate {
  if (state.hasError) return {};

  if (state.justComputed || state.display === '0') {
    return { display: digit, justComputed: false };
  }

  if (state.display.length < 15) {
    return { display: state.display + digit };
  }

  return {};
}

/** Handle decimal point input */
function handleDecimal(state: CalcInternalState): CalcUpdate {
  if (state.display.includes('.') || state.hasError) return {};

  if (state.justComputed) {
    return { display: '0.', justComputed: false };
  }

  return { display: state.display + '.' };
}

/** Handle operator input (+, -, ×, ÷, =) */
function handleOperator(op: string, state: CalcInternalState): CalcUpdate {
  if (state.hasError) return {};

  let newDisplay = state.display;
  const newPendingOp = state.pendingOp;
  let newAccumulator = state.accumulator;

  // If there's a pending operation, compute it first
  if (newPendingOp) {
    const current = parseFloat(newDisplay);
    const result = compute(newAccumulator, current, newPendingOp);
    if (result === null) {
      return { display: 'Error', hasError: true, pendingOp: null };
    }
    newDisplay = formatResult(result);
    newAccumulator = result;
  } else {
    newAccumulator = parseFloat(newDisplay);
  }

  // Set up next operation (unless equals)
  if (op === 'equals') {
    return {
      display: newDisplay,
      pendingOp: null,
      accumulator: newAccumulator,
      justComputed: true,
    };
  }

  return {
    display: newDisplay,
    pendingOp: opToChar(op),
    accumulator: newAccumulator,
    justComputed: true,
  };
}

/** Handle clear (C) - reset all state */
function handleClear(): CalcUpdate {
  return {
    display: '0',
    pendingOp: null,
    hasError: false,
    accumulator: 0,
    justComputed: false,
  };
}

/** Handle clear entry (CE) - reset display only */
function handleClearEntry(): CalcUpdate {
  return { display: '0', hasError: false };
}

/** Handle backspace - remove last character */
function handleBackspace(state: CalcInternalState): CalcUpdate {
  if (state.hasError || state.justComputed) {
    return { display: '0' };
  }

  if (state.display.length > 1) {
    return { display: state.display.slice(0, -1) };
  }

  return { display: '0' };
}

/** Handle negate (±) - toggle sign */
function handleNegate(state: CalcInternalState): CalcUpdate {
  if (state.hasError || state.display === '0') return {};

  const newDisplay = state.display.startsWith('-') ? state.display.slice(1) : '-' + state.display;

  return { display: newDisplay };
}

// =============================================================================
// Calculator Component
// =============================================================================

/**
 * Calculator App - Basic arithmetic calculator
 *
 * Uses ZUI components: Panel, Button, Text, Label
 */
export function CalculatorApp() {
  const [state, setState] = useState<CalculatorState>({
    display: '0',
    pendingOp: null,
    hasError: false,
    memoryIndicator: false,
  });

  const [accumulator, setAccumulator] = useState(0);
  const [justComputed, setJustComputed] = useState(false);

  const handleButton = useCallback(
    (buttonName: string) => {
      // Build internal state for handlers
      const internalState: CalcInternalState = {
        display: state.display,
        pendingOp: state.pendingOp,
        hasError: state.hasError,
        accumulator,
        justComputed,
      };

      // Route to appropriate handler
      let update: CalcUpdate = {};

      if (buttonName.startsWith('digit_')) {
        update = handleDigit(buttonName.replace('digit_', ''), internalState);
      } else if (buttonName === 'decimal') {
        update = handleDecimal(internalState);
      } else if (buttonName.startsWith('op_')) {
        update = handleOperator(buttonName.replace('op_', ''), internalState);
      } else if (buttonName === 'clear') {
        update = handleClear();
      } else if (buttonName === 'clear_entry') {
        update = handleClearEntry();
      } else if (buttonName === 'backspace') {
        update = handleBackspace(internalState);
      } else if (buttonName === 'negate') {
        update = handleNegate(internalState);
      }

      // Apply updates
      if (Object.keys(update).length > 0) {
        setState((prev) => ({
          display: update.display ?? prev.display,
          pendingOp: update.pendingOp !== undefined ? update.pendingOp : prev.pendingOp,
          hasError: update.hasError ?? prev.hasError,
          memoryIndicator: prev.memoryIndicator,
        }));

        if (update.accumulator !== undefined) {
          setAccumulator(update.accumulator);
        }
        if (update.justComputed !== undefined) {
          setJustComputed(update.justComputed);
        }
      }
    },
    [state, accumulator, justComputed]
  );

  const handleMessage = (data: Uint8Array) => {
    const decoded = decodeCalculatorState(data);
    if (decoded) setState(decoded);
  };

  (
    window as unknown as { calculatorAppHandler?: (data: Uint8Array) => void }
  ).calculatorAppHandler = handleMessage;

  return (
    <Panel border="none" className={styles.container}>
      <Panel variant="glass" border="none" className={styles.calculatorPanel}>
        {/* Display */}
        <Panel variant="default" className={styles.displayPanel}>
          <Panel border="none" className={styles.indicators}>
            {state.pendingOp && <Label size="xs">{state.pendingOp}</Label>}
            {state.memoryIndicator && (
              <Label size="xs" variant="warning">
                M
              </Label>
            )}
          </Panel>
          <Text
            as="div"
            size="lg"
            className={state.hasError ? styles.displayError : styles.display}
          >
            {state.display}
          </Text>
        </Panel>

        {/* Keypad */}
        <Panel border="none" className={styles.keypad}>
          {/* Row 1 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('clear_entry')}>
            CE
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('clear')}>
            C
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('backspace')}>
            ⌫
          </Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_div')}>
            ÷
          </Button>

          {/* Row 2 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_7')}>
            7
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_8')}>
            8
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_9')}>
            9
          </Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_mul')}>
            ×
          </Button>

          {/* Row 3 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_4')}>
            4
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_5')}>
            5
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_6')}>
            6
          </Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_sub')}>
            −
          </Button>

          {/* Row 4 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_1')}>
            1
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_2')}>
            2
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_3')}>
            3
          </Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_add')}>
            +
          </Button>

          {/* Row 5 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('negate')}>
            ±
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_0')}>
            0
          </Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('decimal')}>
            .
          </Button>
          <Button variant="primary" size="lg" onClick={() => handleButton('op_equals')}>
            =
          </Button>
        </Panel>
      </Panel>
    </Panel>
  );
}

function opToChar(op: string): string {
  switch (op) {
    case 'add':
      return '+';
    case 'sub':
      return '-';
    case 'mul':
      return '×';
    case 'div':
      return '÷';
    default:
      return op;
  }
}

function compute(a: number, b: number, op: string): number | null {
  switch (op) {
    case '+':
      return a + b;
    case '-':
      return a - b;
    case '×':
    case '*':
      return a * b;
    case '÷':
    case '/':
      return b === 0 ? null : a / b;
    default:
      return b;
  }
}

function formatResult(value: number): string {
  if (Number.isInteger(value) && Math.abs(value) < 1e15) return String(value);
  return value.toFixed(8).replace(/\.?0+$/, '');
}
