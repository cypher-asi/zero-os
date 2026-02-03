import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { Panel, Button, Input, Text, Label, useTheme, type AccentColor } from '@cypher-asi/zui';
import { Send, Paperclip, Circle } from 'lucide-react';
import { getContactById, getMessagesForContact, getStatusColors, type Message } from './types';
import styles from './ConversationWindow.module.css';

// Accent color hex values (must match ZUI theme)
const ACCENT_HEX: Record<AccentColor, string> = {
  cyan: '#01f4cb',
  blue: '#3b82f6',
  purple: '#8b5cf6',
  green: '#22c55e',
  orange: '#f97316',
  rose: '#f43f5e',
};

interface ConversationWindowProps {
  /** The contact ID extracted from appId (e.g., "alice" from "zerochat-conversation-alice") */
  contactId: string;
}

/**
 * Conversation Window - Individual chat window for a contact
 *
 * Uses ZUI components: Panel, Button, Input, Text, Label
 * Layout: Messages area + input bar at bottom
 */
export function ConversationWindow({ contactId }: ConversationWindowProps) {
  const [messages, setMessages] = useState<Message[]>(() => getMessagesForContact(contactId));
  const [inputValue, setInputValue] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const { accent } = useTheme();

  const contact = useMemo(() => getContactById(contactId), [contactId]);
  const statusColors = useMemo(() => getStatusColors(ACCENT_HEX[accent]), [accent]);

  // Scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Handle sending a message
  const handleSend = useCallback(() => {
    if (!inputValue.trim()) return;

    const newMessage: Message = {
      id: `msg-${Date.now()}`,
      senderId: 'me',
      text: inputValue.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, newMessage]);
    setInputValue('');

    // Simulate response after a delay (for demo purposes)
    if (contact?.status === 'online') {
      setTimeout(() => {
        const responses = [
          'Got it!',
          'Sounds good!',
          'Let me check...',
          'Interesting!',
          'Thanks for letting me know!',
        ];
        const responseMessage: Message = {
          id: `msg-${Date.now()}`,
          senderId: contactId,
          text: responses[Math.floor(Math.random() * responses.length)],
          timestamp: new Date(),
        };
        setMessages((prev) => [...prev, responseMessage]);
      }, 1000 + Math.random() * 2000);
    }
  }, [inputValue, contact, contactId]);

  // Handle Enter key
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // Format timestamp
  const formatTime = (date: Date): string => {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  if (!contact) {
    return (
      <Panel border="none" background="none" className={styles.container}>
        <div className={styles.emptyState}>
          <Text variant="muted">Contact not found</Text>
        </div>
      </Panel>
    );
  }

  return (
    <Panel border="none" background="none" className={styles.container}>
      {/* Header with contact info */}
      <div className={styles.header}>
        <div className={styles.contactInfo}>
          <Circle size={10} fill={statusColors[contact.status]} color={statusColors[contact.status]} />
          <Text size="sm" weight="semibold">{contact.name}</Text>
        </div>
        {contact.statusMessage && (
          <Text size="xs" variant="muted" className={styles.statusMessage}>{contact.statusMessage}</Text>
        )}
      </div>

      {/* Messages area */}
      <div className={styles.messagesArea}>
        {messages.length === 0 ? (
          <div className={styles.emptyMessages}>
            <Text variant="muted">No messages yet. Say hello!</Text>
          </div>
        ) : (
          messages.map((message) => (
            <MessageBubble
              key={message.id}
              message={message}
              isOwn={message.senderId === 'me'}
              contactName={contact.name}
              formatTime={formatTime}
            />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input area */}
      <div className={styles.inputArea}>
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          title="Attach file"
        >
          <Paperclip size={16} />
        </Button>
        <Input
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a message..."
          className={styles.messageInput}
        />
        <Button
          variant="primary"
          size="sm"
          iconOnly
          onClick={handleSend}
          disabled={!inputValue.trim()}
        >
          <Send size={16} />
        </Button>
      </div>
    </Panel>
  );
}

interface MessageBubbleProps {
  message: Message;
  isOwn: boolean;
  contactName: string;
  formatTime: (date: Date) => string;
}

function MessageBubble({ message, isOwn, contactName, formatTime }: MessageBubbleProps) {
  return (
    <div className={`${styles.messageBubble} ${isOwn ? styles.own : styles.other}`}>
      <div className={styles.messageHeader}>
        <Text size="xs" weight="semibold">{isOwn ? 'You' : contactName}</Text>
        <Label size="xs">{formatTime(message.timestamp)}</Label>
      </div>
      <Text size="sm">{message.text}</Text>
    </div>
  );
}
