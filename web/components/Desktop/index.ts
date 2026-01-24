/**
 * Desktop Component Module
 *
 * Re-exports the Desktop component and related utilities.
 */

export { Desktop, useBackground } from './Desktop';
export type { DesktopProps, BackgroundInfo } from './types';
// Re-export core types from stores (single source of truth)
export type { WindowInfo, FrameData, ViewportState, WorkspaceInfo } from '../../stores/types';
