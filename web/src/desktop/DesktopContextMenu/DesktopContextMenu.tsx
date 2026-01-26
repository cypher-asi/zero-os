import { useCallback } from 'react';
import {
  Menu,
  THEMES,
  ACCENT_COLORS,
  type Theme,
  type AccentColor,
  type MenuItem,
} from '@cypher-asi/zui';
import styles from '../Desktop/Desktop.module.css';

// Human-readable labels for themes
const THEME_LABELS: Record<Theme, string> = {
  dark: 'Dark',
  light: 'Light',
  system: 'System',
};

// Human-readable labels for accent colors
const ACCENT_LABELS: Record<AccentColor, string> = {
  cyan: 'Cyan',
  blue: 'Blue',
  purple: 'Purple',
  green: 'Green',
  orange: 'Orange',
  rose: 'Rose',
};

export interface BackgroundInfo {
  id: string;
  name: string;
}

export interface DesktopContextMenuProps {
  /** X position of the menu */
  x: number;
  /** Y position of the menu */
  y: number;
  /** Available backgrounds */
  backgrounds: BackgroundInfo[];
  /** Current background ID */
  currentBackground: string;
  /** Current theme */
  theme: Theme;
  /** Current accent color */
  accent: AccentColor;
  /** Callback when background is selected */
  onBackgroundSelect: (id: string) => void;
  /** Callback when theme is selected */
  onThemeSelect: (theme: Theme) => void;
  /** Callback when accent is selected */
  onAccentSelect: (accent: AccentColor) => void;
  /** Callback to close the menu */
  onClose: () => void;
}

/**
 * Desktop Context Menu - Theme, background, and accent color selection
 *
 * Displayed on right-click on the desktop background.
 */
export function DesktopContextMenu({
  x,
  y,
  backgrounds,
  currentBackground,
  theme,
  accent,
  onBackgroundSelect,
  onThemeSelect,
  onAccentSelect,
  onClose,
}: DesktopContextMenuProps) {
  const handleChange = useCallback(
    (id: string) => {
      const [category, value] = id.split(':');
      if (category === 'bg' && value) {
        onBackgroundSelect(value);
        onClose();
      } else if (category === 'theme' && value) {
        onThemeSelect(value as Theme);
        onClose();
      } else if (category === 'accent' && value) {
        onAccentSelect(value as AccentColor);
        onClose();
      }
    },
    [onBackgroundSelect, onThemeSelect, onAccentSelect, onClose]
  );

  const menuItems: MenuItem[] = [
    {
      id: 'background',
      label: 'Background',
      children: backgrounds.map((bg) => ({
        id: `bg:${bg.id}`,
        label: bg.name,
      })),
    },
    {
      id: 'theme',
      label: 'Theme',
      children: THEMES.map((t) => ({
        id: `theme:${t}`,
        label: THEME_LABELS[t],
      })),
    },
    {
      id: 'accent',
      label: 'Accent',
      children: ACCENT_COLORS.map((c) => ({
        id: `accent:${c}`,
        label: ACCENT_LABELS[c],
      })),
    },
  ];

  return (
    <div
      className={styles.contextMenu}
      style={{
        position: 'fixed',
        left: x,
        top: y,
        zIndex: 10000,
      }}
      onClick={(e) => e.stopPropagation()}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <Menu
        items={menuItems}
        value={[`bg:${currentBackground}`, `theme:${theme}`, `accent:${accent}`]}
        onChange={handleChange}
        variant="glass"
        border="future"
        rounded="md"
        width={200}
      />
    </div>
  );
}
