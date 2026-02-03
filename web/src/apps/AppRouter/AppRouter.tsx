import { PageEmptyState } from '@cypher-asi/zui';
import { FolderOpen, Construction } from 'lucide-react';
import { TerminalApp } from '../TerminalApp/TerminalApp';
import { ClockApp } from '../ClockApp/ClockApp';
import { CalculatorApp } from '../CalculatorApp/CalculatorApp';
import { SettingsApp } from '../SettingsApp/SettingsApp';
import { ZeroChatApp, ConversationWindow } from '../ZeroChatApp';

interface AppRouterProps {
  appId: string;
  windowId: number;
  /** Process ID for process-isolated apps like terminal */
  processId?: number;
}

export function AppRouter({ appId, windowId, processId }: AppRouterProps) {
  switch (appId) {
    case 'terminal':
      return <TerminalApp windowId={windowId} processId={processId} />;
    case 'clock':
    case 'com.zero.clock':
      return <ClockApp />;
    case 'calculator':
    case 'com.zero.calculator':
      return <CalculatorApp windowId={windowId} />;
    case 'settings':
    case 'com.zero.settings':
      return <SettingsApp />;
    case 'chat':
    case 'zerochat':
    case 'com.zero.chat':
      return <ZeroChatApp />;
    case 'files':
      return (
        <PageEmptyState
          icon={<FolderOpen size={48} />}
          title="Files"
          description="File manager is not yet implemented"
        />
      );
    default:
      // Handle conversation windows: zerochat-conversation-{contactId}
      if (appId.startsWith('zerochat-conversation-')) {
        const contactId = appId.replace('zerochat-conversation-', '');
        return <ConversationWindow contactId={contactId} />;
      }
      return (
        <PageEmptyState
          icon={<Construction size={48} />}
          title={appId}
          description="This app is not yet implemented"
        />
      );
  }
}
