import { PageEmptyState } from '@cypher-asi/zui';
import { Settings, FolderOpen, Construction } from 'lucide-react';
import { TerminalApp } from '../TerminalApp/TerminalApp';

interface AppRouterProps {
  appId: string;
  windowId: number;
}

export function AppRouter({ appId, windowId }: AppRouterProps) {
  switch (appId) {
    case 'terminal':
      return <TerminalApp windowId={windowId} />;
    case 'settings':
      return (
        <PageEmptyState
          icon={<Settings size={48} />}
          title="Settings"
          description="System settings are not yet implemented"
        />
      );
    case 'files':
      return (
        <PageEmptyState
          icon={<FolderOpen size={48} />}
          title="Files"
          description="File manager is not yet implemented"
        />
      );
    default:
      return (
        <PageEmptyState
          icon={<Construction size={48} />}
          title={appId}
          description="This app is not yet implemented"
        />
      );
  }
}
