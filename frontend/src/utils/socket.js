import { io } from 'socket.io-client';
import { getAuthToken } from './auth';

const SOCKET_URL = process.env.REACT_APP_SOCKET_URL;
const MESSAGING_SOCKET_URL = process.env.REACT_APP_MESSAGING_SOCKET_URL;

/**
 * Create a Socket.IO connection to the matching service (broz-matching:3003)
 */
export function createMatchingSocket(options = {}) {
  const token = getAuthToken();
  if (!token) return null;

  return io(SOCKET_URL, {
    auth: { token },
    query: { token },
    transports: ['websocket', 'polling'],
    reconnection: false,
    timeout: 20000,
    autoConnect: false,
    ...options,
  });
}

/**
 * Create a Socket.IO connection to the messaging service (broz-messaging:3004)
 */
export function createMessagingSocket(options = {}) {
  const token = getAuthToken();
  if (!token) return null;

  return io(MESSAGING_SOCKET_URL, {
    auth: { token },
    query: { token },
    transports: ['websocket', 'polling'],
    reconnection: true,
    reconnectionAttempts: 5,
    reconnectionDelay: 1000,
    autoConnect: true,
    ...options,
  });
}
