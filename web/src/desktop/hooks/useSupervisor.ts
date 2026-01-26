import { createContext, useContext } from 'react';

// Import Supervisor type from shared types (single source of truth)
import type { Supervisor } from '@/shared/types';

// Re-export for backward compatibility
export type { Supervisor } from '@/shared/types';

// =============================================================================
// DesktopController Type - Desktop/Window management (from zero-desktop)
// =============================================================================

export interface DesktopController {
  // Initialization
  init(width: number, height: number): void;
  resize(width: number, height: number): void;

  // Viewport
  pan(dx: number, dy: number): void;
  zoom_at(factor: number, anchor_x: number, anchor_y: number): void;
  get_viewport_json(): string;

  // Windows
  create_window(
    title: string,
    x: number,
    y: number,
    w: number,
    h: number,
    app_id: string,
    content_interactive: boolean
  ): bigint;
  close_window(id: bigint): void;
  get_window_process_id(id: bigint): bigint | undefined;
  /** Link a window to its associated process */
  set_window_process_id(window_id: bigint, process_id: bigint): void;
  focus_window(id: bigint): void;
  move_window(id: bigint, x: number, y: number): void;
  resize_window(id: bigint, w: number, h: number): void;
  minimize_window(id: bigint): void;
  maximize_window(id: bigint): void;
  restore_window(id: bigint): void;
  get_focused_window(): bigint | undefined;
  pan_to_window(id: bigint): void;
  get_windows_json(): string;
  get_window_screen_rects_json(): string;
  launch_app(app_id: string): bigint;

  // Desktops (workspaces)
  create_desktop(name: string): number;
  switch_desktop(index: number): void;
  get_active_desktop(): number;
  get_visual_active_desktop(): number;
  get_desktops_json(): string;
  get_desktop_dimensions_json(): string;

  // Void mode
  get_view_mode(): string;
  is_in_void(): boolean;
  enter_void(): void;
  exit_void(desktop_index: number): void;

  // Animation state
  is_animating(): boolean;
  is_animating_viewport(): boolean;
  is_transitioning(): boolean;
  tick_transition(): boolean;

  // Input handling
  pointer_down(x: number, y: number, button: number, ctrl: boolean, shift: boolean): string;
  pointer_move(x: number, y: number): string;
  pointer_up(): string;
  wheel(dx: number, dy: number, x: number, y: number, ctrl: boolean): string;
  start_window_resize(window_id: bigint, direction: string, x: number, y: number): void;
  start_window_drag(window_id: bigint, x: number, y: number): void;

  // Unified frame tick
  tick_frame(): string;
}

// =============================================================================
// Contexts and Hooks
// =============================================================================

// Supervisor context (kernel operations)
export const SupervisorContext = createContext<Supervisor | null>(null);

// DesktopController context (desktop operations)
export const DesktopControllerContext = createContext<DesktopController | null>(null);

// Hook to access the Supervisor
export function useSupervisor(): Supervisor | null {
  return useContext(SupervisorContext);
}

// Hook to access the DesktopController
export function useDesktopController(): DesktopController | null {
  return useContext(DesktopControllerContext);
}

// Provider components
export const SupervisorProvider = SupervisorContext.Provider;
export const DesktopControllerProvider = DesktopControllerContext.Provider;
