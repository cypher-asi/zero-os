import { useState, useEffect, useCallback } from 'react';
import { useSupervisor } from './useSupervisor';

// Workspace info from Rust
export interface WorkspaceInfo {
  id: number;
  name: string;
  active: boolean;
  windowCount: number;
}

// Hook to get all workspaces
export function useWorkspaces(): WorkspaceInfo[] {
  const supervisor = useSupervisor();
  const [workspaces, setWorkspaces] = useState<WorkspaceInfo[]>([]);

  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      try {
        const json = supervisor.get_workspaces_json();
        const parsed = JSON.parse(json) as WorkspaceInfo[];
        setWorkspaces(parsed);
      } catch (e) {
        console.error('Failed to parse workspaces:', e);
      }
    };

    update();
    const interval = setInterval(update, 200);
    return () => clearInterval(interval);
  }, [supervisor]);

  return workspaces;
}

// Hook to get active workspace index
export function useActiveWorkspace(): number {
  const supervisor = useSupervisor();
  const [active, setActive] = useState(0);

  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      setActive(supervisor.get_active_workspace());
    };

    update();
    const interval = setInterval(update, 200);
    return () => clearInterval(interval);
  }, [supervisor]);

  return active;
}

// Hook for workspace actions
export function useWorkspaceActions() {
  const supervisor = useSupervisor();

  const createWorkspace = useCallback(
    (name: string) => {
      if (!supervisor) return null;
      return supervisor.create_workspace(name);
    },
    [supervisor]
  );

  const switchWorkspace = useCallback(
    (index: number) => {
      supervisor?.switch_workspace(index);
    },
    [supervisor]
  );

  return {
    createWorkspace,
    switchWorkspace,
  };
}
