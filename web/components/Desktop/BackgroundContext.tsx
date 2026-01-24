/**
 * Background Context
 *
 * Provides background controller state to child components.
 */

import { createContext, useContext } from 'react';
import type { BackgroundInfo } from './types';

export interface BackgroundContextType {
  backgrounds: BackgroundInfo[];
  getActiveBackground: () => string;
  setBackground: (id: string) => void;
}

export const BackgroundContext = createContext<BackgroundContextType | null>(null);

export function useBackground(): BackgroundContextType | null {
  return useContext(BackgroundContext);
}
