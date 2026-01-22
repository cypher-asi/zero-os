import { useState, useCallback } from 'react';
import { Panel, Button, Text, Label } from '@cypher-asi/zui';
import { decodeCalculatorState, CalculatorState } from '../shared/app-protocol';
import styles from './CalculatorApp.module.css';

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
      let newDisplay = state.display;
      let newPendingOp = state.pendingOp;
      let newError = false;
      let newAccumulator = accumulator;
      let newJustComputed = justComputed;

      if (buttonName.startsWith('digit_')) {
        const digit = buttonName.replace('digit_', '');
        if (state.hasError) return;
        if (justComputed || newDisplay === '0') {
          newDisplay = digit;
          newJustComputed = false;
        } else if (newDisplay.length < 15) {
          newDisplay = newDisplay + digit;
        }
      } else if (buttonName === 'decimal') {
        if (!newDisplay.includes('.') && !state.hasError) {
          if (justComputed) {
            newDisplay = '0.';
            newJustComputed = false;
          } else {
            newDisplay = newDisplay + '.';
          }
        }
      } else if (buttonName.startsWith('op_')) {
        if (state.hasError) return;
        if (newPendingOp) {
          const current = parseFloat(newDisplay);
          const result = compute(newAccumulator, current, newPendingOp);
          if (result === null) {
            newDisplay = 'Error';
            newError = true;
            newPendingOp = null;
          } else {
            newDisplay = formatResult(result);
            newAccumulator = result;
          }
        } else {
          newAccumulator = parseFloat(newDisplay);
        }
        if (!newError && buttonName !== 'op_equals') {
          newPendingOp = opToChar(buttonName.replace('op_', ''));
          newJustComputed = true;
        } else if (buttonName === 'op_equals') {
          newPendingOp = null;
          newJustComputed = true;
        }
      } else if (buttonName === 'clear') {
        newDisplay = '0';
        newPendingOp = null;
        newError = false;
        newAccumulator = 0;
        newJustComputed = false;
      } else if (buttonName === 'clear_entry') {
        newDisplay = '0';
        newError = false;
      } else if (buttonName === 'backspace') {
        if (!state.hasError && !justComputed && newDisplay.length > 1) {
          newDisplay = newDisplay.slice(0, -1);
        } else {
          newDisplay = '0';
        }
      } else if (buttonName === 'negate') {
        if (!state.hasError && newDisplay !== '0') {
          newDisplay = newDisplay.startsWith('-') ? newDisplay.slice(1) : '-' + newDisplay;
        }
      }

      setState({ display: newDisplay, pendingOp: newPendingOp, hasError: newError, memoryIndicator: state.memoryIndicator });
      setAccumulator(newAccumulator);
      setJustComputed(newJustComputed);
    },
    [state, accumulator, justComputed]
  );

  const handleMessage = (data: Uint8Array) => {
    const decoded = decodeCalculatorState(data);
    if (decoded) setState(decoded);
  };

  (window as unknown as { calculatorAppHandler?: (data: Uint8Array) => void }).calculatorAppHandler = handleMessage;

  return (
    <Panel border="none" className={styles.container}>
      <Panel variant="glass" border="none" className={styles.calculatorPanel}>
        {/* Display */}
        <Panel variant="default" className={styles.displayPanel}>
          <Panel border="none" className={styles.indicators}>
            {state.pendingOp && <Label size="xs">{state.pendingOp}</Label>}
            {state.memoryIndicator && <Label size="xs" variant="warning">M</Label>}
          </Panel>
          <Text as="div" size="lg" className={state.hasError ? styles.displayError : styles.display}>
            {state.display}
          </Text>
        </Panel>

        {/* Keypad */}
        <Panel border="none" className={styles.keypad}>
          {/* Row 1 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('clear_entry')}>CE</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('clear')}>C</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('backspace')}>⌫</Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_div')}>÷</Button>

          {/* Row 2 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_7')}>7</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_8')}>8</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_9')}>9</Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_mul')}>×</Button>

          {/* Row 3 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_4')}>4</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_5')}>5</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_6')}>6</Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_sub')}>−</Button>

          {/* Row 4 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_1')}>1</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_2')}>2</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_3')}>3</Button>
          <Button variant="secondary" size="lg" onClick={() => handleButton('op_add')}>+</Button>

          {/* Row 5 */}
          <Button variant="ghost" size="lg" onClick={() => handleButton('negate')}>±</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('digit_0')}>0</Button>
          <Button variant="ghost" size="lg" onClick={() => handleButton('decimal')}>.</Button>
          <Button variant="primary" size="lg" onClick={() => handleButton('op_equals')}>=</Button>
        </Panel>
      </Panel>
    </Panel>
  );
}

function opToChar(op: string): string {
  switch (op) {
    case 'add': return '+';
    case 'sub': return '-';
    case 'mul': return '×';
    case 'div': return '÷';
    default: return op;
  }
}

function compute(a: number, b: number, op: string): number | null {
  switch (op) {
    case '+': return a + b;
    case '-': return a - b;
    case '×': case '*': return a * b;
    case '÷': case '/': return b === 0 ? null : a / b;
    default: return b;
  }
}

function formatResult(value: number): string {
  if (Number.isInteger(value) && Math.abs(value) < 1e15) return String(value);
  return value.toFixed(8).replace(/\.?0+$/, '');
}
