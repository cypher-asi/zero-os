import { Menu, GroupCollapsible, useTheme, THEMES, ACCENT_COLORS, type Theme, type AccentColor, type MenuItem } from '@cypher-asi/zui';
import { Sun, Moon, Monitor, Image } from 'lucide-react';
import { useBackground } from '../../../components/Desktop/Desktop';
import styles from './panels.module.css';

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

// Accent color hex values for swatches
const ACCENT_HEX: Record<AccentColor, string> = {
  cyan: '#01f4cb',
  blue: '#3b82f6',
  purple: '#8b5cf6',
  green: '#22c55e',
  orange: '#f97316',
  rose: '#f43f5e',
};

// Theme icons
const THEME_ICONS: Record<Theme, typeof Moon> = {
  dark: Moon,
  light: Sun,
  system: Monitor,
};

/**
 * Theme Settings Panel
 * - Theme mode (dark/light/system)
 * - Accent color selection
 * - Background pattern selection
 * 
 * Wired up to actual ZUI theme context and Desktop background context.
 */
export function ThemePanel() {
  const { theme, accent, setTheme, setAccent } = useTheme();
  const backgroundCtx = useBackground();
  
  const backgrounds = backgroundCtx?.backgrounds ?? [];
  const currentBackground = backgroundCtx?.getActiveBackground() ?? 'grain';
  const setBackground = backgroundCtx?.setBackground;

  // Theme mode menu items
  const themeItems: MenuItem[] = THEMES.map((t) => {
    const Icon = THEME_ICONS[t];
    return {
      id: t,
      label: THEME_LABELS[t],
      icon: <Icon size={14} />,
    };
  });

  // Accent color menu items
  const accentItems: MenuItem[] = ACCENT_COLORS.map((color) => ({
    id: color,
    label: ACCENT_LABELS[color],
    icon: (
      <span
        className={styles.colorDot}
        style={{ backgroundColor: ACCENT_HEX[color] }}
      />
    ),
  }));

  // Background menu items
  const backgroundItems: MenuItem[] = backgrounds.map((bg) => ({
    id: bg.id,
    label: bg.name,
    icon: <Image size={14} />,
  }));

  return (
    <div className={styles.panelContainer}>
      <GroupCollapsible
        title="Appearance"
        count={THEMES.length}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={themeItems} value={theme} onChange={(id) => setTheme(id as Theme)} background="none" border="none" />
        </div>
      </GroupCollapsible>

      <GroupCollapsible
        title="Accent Color"
        count={ACCENT_COLORS.length}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={accentItems} value={accent} onChange={(id) => setAccent(id as AccentColor)} background="none" border="none" />
        </div>
      </GroupCollapsible>

      <GroupCollapsible
        title="Background"
        count={backgrounds.length}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={backgroundItems} value={currentBackground} onChange={(id) => setBackground?.(id)} background="none" border="none" />
        </div>
      </GroupCollapsible>
    </div>
  );
}
