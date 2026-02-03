import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { CalculatorApp } from './CalculatorApp';

// Mock the window store
const mockFocusedId = vi.fn(() => 1);
vi.mock('../../stores/windowStore', () => ({
  useWindowStore: (selector: (state: { focusedId: number | null }) => unknown) =>
    selector({ focusedId: mockFocusedId() }),
  selectFocusedId: (state: { focusedId: number | null }) => state.focusedId,
}));

// Mock @cypher-asi/zui components
vi.mock('@cypher-asi/zui', () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Panel: ({ children, className, ...props }: Record<string, any>) => (
    <div className={className} {...props}>
      {children}
    </div>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Button: ({ children, onClick, variant, ...props }: Record<string, any>) => (
    <button onClick={onClick} data-variant={variant} data-testid={`btn-${children}`} {...props}>
      {children}
    </button>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Text: ({ children, as: Component = 'span', className, ...props }: Record<string, any>) => (
    <Component className={className} data-testid="display" {...props}>
      {children}
    </Component>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Label: ({ children, variant, ...props }: Record<string, any>) => (
    <span data-variant={variant} {...props}>
      {children}
    </span>
  ),
}));

// Helper to get the display value
function getDisplay() {
  return screen.getByTestId('display').textContent;
}

// Helper to click a button
function clickButton(label: string) {
  fireEvent.click(screen.getByTestId(`btn-${label}`));
}

// Helper to simulate keyboard input
function pressKey(key: string) {
  fireEvent.keyDown(window, { key });
}

// Default windowId for tests
const TEST_WINDOW_ID = 1;

describe('CalculatorApp', () => {
  beforeEach(() => {
    // Clean up any global handlers
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (window as Record<string, any>).calculatorAppHandler;
    // Reset mock to default focused state
    mockFocusedId.mockReturnValue(TEST_WINDOW_ID);
  });

  describe('Rendering', () => {
    it('renders with initial display of 0', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      expect(getDisplay()).toBe('0');
    });

    it('renders all digit buttons', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      for (let i = 0; i <= 9; i++) {
        // getByTestId throws if not found, so if we get here it exists
        expect(screen.getByTestId(`btn-${i}`)).toBeDefined();
      }
    });

    it('renders operator buttons', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      expect(screen.getByTestId('btn-+')).toBeDefined();
      expect(screen.getByTestId('btn-−')).toBeDefined();
      expect(screen.getByTestId('btn-×')).toBeDefined();
      expect(screen.getByTestId('btn-÷')).toBeDefined();
      expect(screen.getByTestId('btn-=')).toBeDefined();
    });

    it('renders utility buttons', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      expect(screen.getByTestId('btn-C')).toBeDefined();
      expect(screen.getByTestId('btn-CE')).toBeDefined();
      expect(screen.getByTestId('btn-⌫')).toBeDefined();
      expect(screen.getByTestId('btn-±')).toBeDefined();
      expect(screen.getByTestId('btn-.')).toBeDefined();
    });
  });

  describe('Digit Input', () => {
    it('displays single digit when pressed', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      expect(getDisplay()).toBe('5');
    });

    it('appends digits to form multi-digit numbers', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('1');
      clickButton('2');
      clickButton('3');
      expect(getDisplay()).toBe('123');
    });

    it('replaces initial 0 with first digit', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('7');
      expect(getDisplay()).toBe('7');
    });
  });

  describe('Decimal Input', () => {
    it('adds decimal point to display', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('3');
      clickButton('.');
      clickButton('1');
      clickButton('4');
      expect(getDisplay()).toBe('3.14');
    });

    it('adds leading zero when decimal pressed first', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('.');
      clickButton('5');
      expect(getDisplay()).toBe('0.5');
    });

    it('ignores second decimal point', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('3');
      clickButton('.');
      clickButton('1');
      clickButton('.');
      clickButton('4');
      expect(getDisplay()).toBe('3.14');
    });
  });

  describe('Basic Operations', () => {
    it('performs addition', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('2');
      clickButton('+');
      clickButton('3');
      clickButton('=');
      expect(getDisplay()).toBe('5');
    });

    it('performs subtraction', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('7');
      clickButton('−');
      clickButton('4');
      clickButton('=');
      expect(getDisplay()).toBe('3');
    });

    it('performs multiplication', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('6');
      clickButton('×');
      clickButton('7');
      clickButton('=');
      expect(getDisplay()).toBe('42');
    });

    it('performs division', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('8');
      clickButton('÷');
      clickButton('2');
      clickButton('=');
      expect(getDisplay()).toBe('4');
    });

    it('shows error on division by zero', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('÷');
      clickButton('0');
      clickButton('=');
      expect(getDisplay()).toBe('Error');
    });
  });

  describe('Chained Operations', () => {
    it('handles chained operations without equals', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      // 2 + 3 + 4 = 9
      clickButton('2');
      clickButton('+');
      clickButton('3');
      clickButton('+');
      // After second + the display should show 5
      expect(getDisplay()).toBe('5');
      clickButton('4');
      clickButton('=');
      expect(getDisplay()).toBe('9');
    });
  });

  describe('Clear Functions', () => {
    it('clears all with C button', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('+');
      clickButton('3');
      clickButton('C');
      expect(getDisplay()).toBe('0');
    });

    it('clears entry with CE button', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('+');
      clickButton('3');
      clickButton('CE');
      // Display should be 0 but pending operation preserved
      expect(getDisplay()).toBe('0');
    });

    it('clears error state with C button', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('÷');
      clickButton('0');
      clickButton('=');
      expect(getDisplay()).toBe('Error');

      clickButton('C');
      expect(getDisplay()).toBe('0');
    });
  });

  describe('Backspace', () => {
    it('removes last digit with backspace', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('1');
      clickButton('2');
      clickButton('3');
      clickButton('⌫');
      expect(getDisplay()).toBe('12');
    });

    it('shows 0 when backspace on single digit', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('⌫');
      expect(getDisplay()).toBe('0');
    });
  });

  describe('Negate', () => {
    it('negates positive number', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('±');
      expect(getDisplay()).toBe('-5');
    });

    it('makes negative number positive', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('5');
      clickButton('±');
      clickButton('±');
      expect(getDisplay()).toBe('5');
    });

    it('does not negate zero', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('±');
      expect(getDisplay()).toBe('0');
    });
  });

  describe('State After Computation', () => {
    it('starts new number after equals', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      clickButton('2');
      clickButton('+');
      clickButton('3');
      clickButton('=');
      expect(getDisplay()).toBe('5');

      // New number should replace the result
      clickButton('9');
      expect(getDisplay()).toBe('9');
    });
  });

  describe('External Message Handler', () => {
    it('registers global handler', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect((window as Record<string, any>).calculatorAppHandler).toBeDefined();
    });
  });

  describe('Keyboard Input', () => {
    it('handles digit keys', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      expect(getDisplay()).toBe('5');
    });

    it('handles multiple digit keys', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('1');
      pressKey('2');
      pressKey('3');
      expect(getDisplay()).toBe('123');
    });

    it('handles decimal key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('3');
      pressKey('.');
      pressKey('1');
      pressKey('4');
      expect(getDisplay()).toBe('3.14');
    });

    it('handles addition with + key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('2');
      pressKey('+');
      pressKey('3');
      pressKey('Enter');
      expect(getDisplay()).toBe('5');
    });

    it('handles subtraction with - key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('7');
      pressKey('-');
      pressKey('4');
      pressKey('=');
      expect(getDisplay()).toBe('3');
    });

    it('handles multiplication with * key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('6');
      pressKey('*');
      pressKey('7');
      pressKey('Enter');
      expect(getDisplay()).toBe('42');
    });

    it('handles division with / key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('8');
      pressKey('/');
      pressKey('2');
      pressKey('Enter');
      expect(getDisplay()).toBe('4');
    });

    it('handles equals with = key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      pressKey('+');
      pressKey('5');
      pressKey('=');
      expect(getDisplay()).toBe('10');
    });

    it('handles Backspace key', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('1');
      pressKey('2');
      pressKey('3');
      pressKey('Backspace');
      expect(getDisplay()).toBe('12');
    });

    it('handles Escape key for clear', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      pressKey('+');
      pressKey('3');
      pressKey('Escape');
      expect(getDisplay()).toBe('0');
    });

    it('handles c key for clear', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      pressKey('+');
      pressKey('3');
      pressKey('c');
      expect(getDisplay()).toBe('0');
    });

    it('handles C key for clear', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('9');
      pressKey('C');
      expect(getDisplay()).toBe('0');
    });

    it('handles Delete key for clear entry', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      pressKey('+');
      pressKey('3');
      pressKey('Delete');
      expect(getDisplay()).toBe('0');
    });

    it('ignores keyboard input when window is not focused', () => {
      mockFocusedId.mockReturnValue(999); // Different window focused
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      expect(getDisplay()).toBe('0'); // Should remain at initial state
    });

    it('processes keyboard input when window becomes focused', () => {
      mockFocusedId.mockReturnValue(999); // Different window focused
      const { rerender } = render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      expect(getDisplay()).toBe('0'); // Ignored

      mockFocusedId.mockReturnValue(TEST_WINDOW_ID); // Now focused
      rerender(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('3');
      expect(getDisplay()).toBe('3'); // Processed
    });

    it('ignores unrecognized keys', () => {
      render(<CalculatorApp windowId={TEST_WINDOW_ID} />);
      pressKey('5');
      pressKey('a'); // Not a calculator key
      pressKey('!'); // Not a calculator key
      pressKey('Tab'); // Not a calculator key
      expect(getDisplay()).toBe('5');
    });
  });
});
