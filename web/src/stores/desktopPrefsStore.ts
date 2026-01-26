/**
 * Desktop Preferences Store
 *
 * Persists user preferences to localStorage:
 * - Active workspace index
 * - Per-workspace background selections
 *
 * Note: Theme and accent color are already persisted by @cypher-asi/zui's ThemeProvider
 * to localStorage key 'zui-theme'.
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface DesktopPrefsState {
  /** Currently active workspace index */
  activeWorkspace: number;
  /** Per-workspace background selections (workspace index -> background id) */
  backgrounds: Record<number, string>;
  /** Set the active workspace index */
  setActiveWorkspace: (index: number) => void;
  /** Set background for a specific workspace */
  setBackground: (workspaceIndex: number, backgroundId: string) => void;
  /** Get background for a specific workspace (defaults to 'grain') */
  getBackground: (workspaceIndex: number) => string;
}

export const useDesktopPrefsStore = create<DesktopPrefsState>()(
  persist(
    (set, get) => ({
      activeWorkspace: 0,
      backgrounds: { 0: 'grain' },
      setActiveWorkspace: (index) => set({ activeWorkspace: index }),
      setBackground: (workspaceIndex, backgroundId) =>
        set((state) => ({
          backgrounds: { ...state.backgrounds, [workspaceIndex]: backgroundId },
        })),
      getBackground: (workspaceIndex) => get().backgrounds[workspaceIndex] ?? 'grain',
    }),
    { name: 'zero-desktop-prefs' }
  )
);

// Selectors for convenience
export const selectActiveWorkspace = (state: DesktopPrefsState): number => state.activeWorkspace;
export const selectBackgrounds = (state: DesktopPrefsState): Record<number, string> =>
  state.backgrounds;
