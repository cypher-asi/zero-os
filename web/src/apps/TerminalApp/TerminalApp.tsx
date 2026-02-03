import { useRef, useEffect, useState, useCallback } from 'react';
import { useSupervisor } from '@desktop/hooks/useSupervisor';
import styles from './TerminalApp.module.css';

interface TerminalAppProps {
  windowId: number;
  /** Process ID for this terminal instance (for process isolation) */
  processId?: number;
}

export function TerminalApp({ windowId: _windowId, processId }: TerminalAppProps) {
  const supervisor = useSupervisor();
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [output, setOutput] = useState<Array<{ text: string; className: string }>>([]);
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);

  // Set up console callback - either per-process (if processId provided) or legacy global
  useEffect(() => {
    console.log(
      '[TerminalApp] useEffect running, supervisor:',
      supervisor ? 'available' : 'null',
      'processId:',
      processId
    );
    if (!supervisor) return;

    const handleOutput = (text: string) => {
      console.log('[TerminalApp] Received:', JSON.stringify(text), 'processId:', processId);
      // Handle clear screen escape sequence
      if (text.includes('\x1B[2J')) {
        setOutput([]);
        return;
      }
      setOutput((prev) => [...prev, { text, className: styles.outputText }]);
    };

    if (processId != null) {
      // Process-isolated mode: register callback for specific PID
      // Note: Rust u64 maps to JavaScript BigInt in wasm-bindgen
      console.log('[TerminalApp] Registering per-process callback for PID', processId);
      supervisor.register_console_callback(BigInt(processId), handleOutput);

      // Cleanup: unregister callback when unmounting
      return () => {
        console.log('[TerminalApp] Unregistering callback for PID', processId);
        supervisor.unregister_console_callback(BigInt(processId));
      };
    } else {
      // No processId available - console callbacks are per-process, so we can't register
      console.warn('[TerminalApp] No processId - console output will be buffered');
    }
  }, [supervisor, processId]);

  // Scroll to bottom on new output
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [output]);

  // Auto-focus input when terminal opens
  useEffect(() => {
    // Small delay to ensure the window is fully rendered
    const timer = setTimeout(() => {
      inputRef.current?.focus();
    }, 50);
    return () => clearTimeout(timer);
  }, []);

  const handleSubmit = useCallback(() => {
    const input = inputRef.current;
    if (!input || !supervisor) return;

    const line = input.value;
    input.value = '';

    if (line.trim()) {
      setCommandHistory((prev) => [...prev, line]);
      setHistoryIndex(-1);
    }

    setOutput((prev) => [...prev, { text: `z::> ${line}\n`, className: styles.inputEcho }]);

    // Send input to specific process (if processId provided) or legacy global
    if (processId != null) {
      supervisor.send_input_to_process(BigInt(processId), line);
    } else {
      supervisor.send_input(line);
    }
  }, [supervisor, processId]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') {
        handleSubmit();
      } else if (e.key === 'Escape') {
        // Blur the input to allow desktop keyboard shortcuts
        e.preventDefault();
        if (inputRef.current) {
          inputRef.current.blur();
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (commandHistory.length > 0) {
          const newIndex =
            historyIndex < 0 ? commandHistory.length - 1 : Math.max(0, historyIndex - 1);
          setHistoryIndex(newIndex);
          if (inputRef.current) {
            inputRef.current.value = commandHistory[newIndex];
          }
        }
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (historyIndex >= 0) {
          const newIndex = historyIndex + 1;
          if (newIndex >= commandHistory.length) {
            setHistoryIndex(-1);
            if (inputRef.current) inputRef.current.value = '';
          } else {
            setHistoryIndex(newIndex);
            if (inputRef.current) inputRef.current.value = commandHistory[newIndex];
          }
        }
      }
    },
    [commandHistory, historyIndex, handleSubmit]
  );

  return (
    <div className={styles.terminalApp}>
      <div
        ref={terminalRef}
        className={styles.terminal}
        data-selectable-text
        onClick={() => {
          // Only focus input if no text is selected (preserve text selection)
          const selection = window.getSelection();
          if (!selection || selection.isCollapsed) {
            inputRef.current?.focus();
          }
        }}
      >
        {output.map((line, i) => (
          <span key={i} className={line.className}>
            {line.text}
          </span>
        ))}
        <span className={styles.inputLine}>
          <span className={styles.prompt}>z::&gt;</span>
          <input
            ref={inputRef}
            type="text"
            className={styles.input}
            autoComplete="off"
            spellCheck={false}
            onKeyDown={handleKeyDown}
          />
        </span>
      </div>
    </div>
  );
}
