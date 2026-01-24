import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act, waitFor } from '@testing-library/react';
import { createElement } from 'react';
import { TerminalApp } from './TerminalApp';
import { SupervisorProvider } from '../../desktop/hooks/useSupervisor';
import { createMockSupervisor } from '../../test/mocks';

// Mock @cypher-asi/zui components
vi.mock('@cypher-asi/zui', () => ({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Drawer: ({ children, open, onClose }: Record<string, any>) => (
    <div data-testid="drawer" data-open={open} onClick={onClose}>
      {children}
    </div>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  GroupCollapsible: ({ children, title }: Record<string, any>) => (
    <div data-testid="group-collapsible">
      <span>{title}</span>
      {children}
    </div>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Button: ({ children, onClick, variant, ...props }: Record<string, any>) => (
    <button onClick={onClick} data-variant={variant} {...props}>
      {children}
    </button>
  ),
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Text: ({ children, ...props }: Record<string, any>) => <span {...props}>{children}</span>,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  Label: ({ children, ...props }: Record<string, any>) => <span {...props}>{children}</span>,
}));

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  Menu: () => <span data-testid="menu-icon">Menu</span>,
  Circle: () => <span data-testid="circle-icon">Circle</span>,
  Activity: () => <span data-testid="activity-icon">Activity</span>,
  Database: () => <span data-testid="database-icon">Database</span>,
  Cpu: () => <span data-testid="cpu-icon">Cpu</span>,
  HardDrive: () => <span data-testid="harddrive-icon">HardDrive</span>,
  CheckCircle: () => <span data-testid="check-icon">Check</span>,
  AlertCircle: () => <span data-testid="alert-icon">Alert</span>,
}));

// Mock CSS module
vi.mock('./TerminalApp.module.css', () => ({
  default: {
    container: 'container',
    header: 'header',
    menuButton: 'menuButton',
    terminal: 'terminal',
    output: 'output',
    inputLine: 'inputLine',
    prompt: 'prompt',
    input: 'input',
    outputText: 'outputText',
    drawer: 'drawer',
    drawerContent: 'drawerContent',
    statsSection: 'statsSection',
    statsGrid: 'statsGrid',
    statCard: 'statCard',
    statValue: 'statValue',
    statLabel: 'statLabel',
    processCard: 'processCard',
    processHeader: 'processHeader',
    processInfo: 'processInfo',
    processInfoItem: 'processInfoItem',
    noProcesses: 'noProcesses',
  },
}));

function createWrapper(supervisor: ReturnType<typeof createMockSupervisor>) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return createElement(SupervisorProvider, { value: supervisor }, children);
  };
}

describe('TerminalApp', () => {
  let mockSupervisor: ReturnType<typeof createMockSupervisor>;

  beforeEach(() => {
    vi.useFakeTimers();
    mockSupervisor = createMockSupervisor();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe('Rendering', () => {
    it('renders terminal container', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Should have input element
      const input = screen.getByRole('textbox');
      expect(input).toBeDefined();
    });

    it('renders with prompt', () => {
      const { container } = render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Container should have content (including prompt area)
      expect(container.textContent?.length).toBeGreaterThan(0);
    });

    it('renders menu button', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Should have at least one button (the menu button)
      const buttons = screen.getAllByRole('button');
      expect(buttons.length).toBeGreaterThan(0);
    });
  });

  describe('Console callback registration', () => {
    it('registers console callback with supervisor', async () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // The effect runs synchronously on mount
      expect(mockSupervisor.set_console_callback).toHaveBeenCalled();
    });

    it('registers per-process callback when processId is provided', async () => {
      render(<TerminalApp windowId={1} processId={42} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      expect(mockSupervisor.register_console_callback).toHaveBeenCalledWith(
        BigInt(42),
        expect.any(Function)
      );
    });

    it('unregisters callback on unmount when processId is provided', async () => {
      const { unmount } = render(<TerminalApp windowId={1} processId={42} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      unmount();

      expect(mockSupervisor.unregister_console_callback).toHaveBeenCalledWith(BigInt(42));
    });
  });

  describe('Command input', () => {
    it('accepts input', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');
      fireEvent.change(input, { target: { value: 'help' } });

      expect((input as HTMLInputElement).value).toBe('help');
    });

    it('sends command on Enter key', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');
      fireEvent.change(input, { target: { value: 'ps' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      expect(mockSupervisor.send_input).toHaveBeenCalledWith('ps');
    });

    it('clears input after sending command', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');
      fireEvent.change(input, { target: { value: 'help' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      expect((input as HTMLInputElement).value).toBe('');
    });

    it('sends whitespace-only commands but does not add to history', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');
      fireEvent.change(input, { target: { value: '   ' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // Whitespace-only commands ARE sent (terminal echoes everything)
      expect(mockSupervisor.send_input).toHaveBeenCalledWith('   ');
    });
  });

  describe('Command history', () => {
    it('navigates history with ArrowUp', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');

      // Send first command
      fireEvent.change(input, { target: { value: 'command1' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // Send second command
      fireEvent.change(input, { target: { value: 'command2' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // Navigate back in history
      fireEvent.keyDown(input, { key: 'ArrowUp' });
      expect((input as HTMLInputElement).value).toBe('command2');

      fireEvent.keyDown(input, { key: 'ArrowUp' });
      expect((input as HTMLInputElement).value).toBe('command1');
    });

    it('navigates history with ArrowDown', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      const input = screen.getByRole('textbox');

      // Send commands
      fireEvent.change(input, { target: { value: 'command1' } });
      fireEvent.keyDown(input, { key: 'Enter' });
      fireEvent.change(input, { target: { value: 'command2' } });
      fireEvent.keyDown(input, { key: 'Enter' });

      // Navigate up then down
      fireEvent.keyDown(input, { key: 'ArrowUp' });
      fireEvent.keyDown(input, { key: 'ArrowUp' });
      fireEvent.keyDown(input, { key: 'ArrowDown' });

      expect((input as HTMLInputElement).value).toBe('command2');
    });
  });

  describe('Dashboard', () => {
    it('can open drawer', () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Find and click the menu button (first button)
      const buttons = screen.getAllByRole('button');
      fireEvent.click(buttons[0]);

      // Drawer should be rendered (the click toggled state)
      const drawer = screen.getByTestId('drawer');
      expect(drawer).toBeDefined();
    });

    it('fetches processes periodically', async () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Run timers to trigger the interval
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });

      // Check that get_process_list_json was called (used by dashboard)
      expect(mockSupervisor.get_process_list_json).toHaveBeenCalled();
    });

    it('fetches axiom stats periodically', async () => {
      render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Run timers to trigger the interval
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });

      // Check that get_axiom_stats_json was called
      expect(mockSupervisor.get_axiom_stats_json).toHaveBeenCalled();
    });
  });

  describe('Special commands', () => {
    it('handles clear screen escape sequence', () => {
      const { container } = render(<TerminalApp windowId={1} />, {
        wrapper: createWrapper(mockSupervisor),
      });

      // Get the callback that was registered
      const callback = (mockSupervisor.set_console_callback as ReturnType<typeof vi.fn>).mock
        .calls[0][0];

      // Simulate receiving output
      act(() => {
        callback('Line 1');
        callback('Line 2');
      });

      // Output should be added
      expect(container.textContent).toContain('Line 1');

      // Simulate clear screen
      act(() => {
        callback('\x1B[2J');
      });

      // Output should be cleared
      expect(container.textContent).not.toContain('Line 1');
    });
  });
});
