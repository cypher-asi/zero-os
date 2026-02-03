import type { ExplorerNode } from '@cypher-asi/zui';
import { Users, Briefcase, Home } from 'lucide-react';
import { createElement } from 'react';

export type ContactStatus = 'online' | 'away' | 'offline';

export interface Contact {
  id: string;
  name: string;
  status: ContactStatus;
  avatar?: string;
  statusMessage?: string;
}

export interface Message {
  id: string;
  senderId: string;
  text: string;
  timestamp: Date;
}

export interface Conversation {
  contactId: string;
  messages: Message[];
}

// Status indicator colors (online uses theme accent, passed at runtime)
export const STATUS_COLORS: Record<ContactStatus, string> = {
  online: '#01f4cb',  // default accent (overridden at runtime)
  away: '#f59e0b',    // amber
  offline: '#6b7280', // gray
};

// Get status colors with theme accent for online
export function getStatusColors(accentColor?: string): Record<ContactStatus, string> {
  return {
    online: accentColor || '#01f4cb',
    away: '#f59e0b',
    offline: '#6b7280',
  };
}

// Mock contacts data
export const MOCK_CONTACTS: Contact[] = [
  // Friends
  { id: 'alice', name: 'Alice Chen', status: 'online', statusMessage: 'Working on ZeroOS' },
  { id: 'bob', name: 'Bob Smith', status: 'away', statusMessage: 'Be right back' },
  { id: 'carol', name: 'Carol Wu', status: 'offline' },
  // Work
  { id: 'emma', name: 'Emma Johnson', status: 'online', statusMessage: 'In a meeting' },
  { id: 'frank', name: 'Frank Miller', status: 'online' },
  // Family
  { id: 'grace', name: 'Grace Lee', status: 'offline' },
  { id: 'henry', name: 'Henry Park', status: 'away', statusMessage: 'On vacation' },
];

// Category icons
const CATEGORY_ICONS = {
  friends: Users,
  work: Briefcase,
  family: Home,
};

// Contact categories for Explorer
export const CONTACT_CATEGORIES = [
  { id: 'friends', label: 'Friends', contactIds: ['alice', 'bob', 'carol'] },
  { id: 'work', label: 'Work', contactIds: ['emma', 'frank'] },
  { id: 'family', label: 'Family', contactIds: ['grace', 'henry'] },
];

// Helper to get contact by ID
export function getContactById(id: string): Contact | undefined {
  return MOCK_CONTACTS.find((c) => c.id === id);
}

// Helper to count online contacts in a category
export function countOnlineInCategory(contactIds: string[]): number {
  return contactIds.filter((id) => {
    const contact = getContactById(id);
    return contact?.status === 'online';
  }).length;
}

// Build Explorer data from contacts (file-explorer style)
// Status is stored in metadata for custom rendering with indicator on right
export function buildContactExplorerData(): ExplorerNode[] {
  return CONTACT_CATEGORIES.map((category) => {
    const onlineCount = countOnlineInCategory(category.contactIds);
    const totalCount = category.contactIds.length;
    const CategoryIcon = CATEGORY_ICONS[category.id as keyof typeof CATEGORY_ICONS];

    return {
      id: category.id,
      label: `${category.label} (${onlineCount}/${totalCount})`,
      icon: createElement(CategoryIcon, { size: 14 }),
      children: category.contactIds.map((contactId) => {
        const contact = getContactById(contactId);
        if (!contact) {
          return { id: `contact-${contactId}`, label: contactId };
        }
        return {
          id: `contact-${contact.id}`,
          label: contact.name,
          metadata: { status: contact.status },
        };
      }),
    };
  });
}

// Mock conversation data
export const MOCK_CONVERSATIONS: Record<string, Message[]> = {
  alice: [
    { id: '1', senderId: 'alice', text: 'Hey! How are you?', timestamp: new Date(Date.now() - 3600000) },
    { id: '2', senderId: 'me', text: 'Doing great! Working on the new chat app', timestamp: new Date(Date.now() - 3500000) },
    { id: '3', senderId: 'alice', text: 'Nice! Let me know if you need any help', timestamp: new Date(Date.now() - 3400000) },
    { id: '4', senderId: 'me', text: 'Will do, thanks!', timestamp: new Date(Date.now() - 3300000) },
  ],
  bob: [
    { id: '1', senderId: 'bob', text: 'Did you see the game last night?', timestamp: new Date(Date.now() - 86400000) },
    { id: '2', senderId: 'me', text: 'Yeah it was amazing!', timestamp: new Date(Date.now() - 86300000) },
  ],
  emma: [
    { id: '1', senderId: 'emma', text: 'The project deadline is Friday', timestamp: new Date(Date.now() - 7200000) },
    { id: '2', senderId: 'me', text: 'Got it, I\'ll have my part done by Thursday', timestamp: new Date(Date.now() - 7100000) },
    { id: '3', senderId: 'emma', text: 'Perfect, thanks!', timestamp: new Date(Date.now() - 7000000) },
  ],
  frank: [
    { id: '1', senderId: 'me', text: 'Hey Frank, quick question about the API', timestamp: new Date(Date.now() - 1800000) },
  ],
};

// Get messages for a contact
export function getMessagesForContact(contactId: string): Message[] {
  return MOCK_CONVERSATIONS[contactId] || [];
}
