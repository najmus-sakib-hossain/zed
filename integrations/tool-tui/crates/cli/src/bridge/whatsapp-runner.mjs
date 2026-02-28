// WhatsApp Bridge Runner
// Uses @whiskeysockets/baileys for WhatsApp Web protocol
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import makeWASocket, {
  DisconnectReason,
  useMultiFileAuthState,
  fetchLatestBaileysVersion,
  makeCacheableSignalKeyStore,
} from '@whiskeysockets/baileys';
import pino from 'pino';
import * as qrcode from 'qrcode-terminal';

// Logger
const logger = pino({ level: 'silent' });

// State
let sock = null;
let authState = null;
let saveCreds = null;

// Send JSON response to stdout
function sendResponse(id, result = null, error = null) {
  const response = { id };
  if (result !== null) response.result = result;
  if (error !== null) response.error = error;
  console.log(JSON.stringify(response));
}

// Send event to stdout
function sendEvent(event, payload) {
  console.log(JSON.stringify({ event, payload }));
}

// Initialize WhatsApp connection
async function init() {
  try {
    const authDir = process.env.AUTH_DIR || './auth/whatsapp';
    const { state, saveCreds: sc } = await useMultiFileAuthState(authDir);
    authState = state;
    saveCreds = sc;

    const { version } = await fetchLatestBaileysVersion();

    sock = makeWASocket({
      version,
      auth: {
        creds: state.creds,
        keys: makeCacheableSignalKeyStore(state.keys, logger),
      },
      printQRInTerminal: false,
      logger,
      generateHighQualityLinkPreview: true,
    });

    // Handle connection updates
    sock.ev.on('connection.update', (update) => {
      const { connection, lastDisconnect, qr } = update;

      if (qr) {
        // Send QR code event
        sendEvent('qr', { qr });
        qrcode.generate(qr, { small: true });
      }

      if (connection === 'close') {
        const shouldReconnect =
          lastDisconnect?.error?.output?.statusCode !== DisconnectReason.loggedOut;

        sendEvent('state', {
          state: 'disconnected',
          reason: lastDisconnect?.error?.message,
          shouldReconnect,
        });

        if (shouldReconnect) {
          setTimeout(() => init(), 3000);
        }
      } else if (connection === 'open') {
        sendEvent('state', { state: 'connected' });
      }
    });

    // Save credentials on update
    sock.ev.on('creds.update', saveCreds);

    // Handle incoming messages
    sock.ev.on('messages.upsert', async ({ messages, type }) => {
      for (const msg of messages) {
        if (!msg.message || msg.key.fromMe) continue;

        const text =
          msg.message.conversation ||
          msg.message.extendedTextMessage?.text ||
          '';

        sendEvent('message', {
          id: msg.key.id,
          from: msg.key.remoteJid,
          participant: msg.key.participant,
          text,
          timestamp: msg.messageTimestamp,
          type,
          raw: msg,
        });
      }
    });

    // Handle message receipts (read, delivered)
    sock.ev.on('messages.update', (updates) => {
      for (const update of updates) {
        sendEvent('message.update', {
          id: update.key.id,
          chat: update.key.remoteJid,
          status: update.update.status,
        });
      }
    });

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a message
async function send({ to, message, options = {} }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    // Ensure JID format
    const jid = to.includes('@') ? to : `${to}@s.whatsapp.net`;

    const result = await sock.sendMessage(jid, { text: message }, options);

    return {
      success: true,
      id: result.key.id,
      timestamp: result.messageTimestamp,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send media
async function sendMedia({ to, url, caption, type = 'image', gifPlayback = false }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const jid = to.includes('@') ? to : `${to}@s.whatsapp.net`;

    let content;
    if (type === 'image') {
      content = { image: { url }, caption, gifPlayback };
    } else if (type === 'video') {
      content = { video: { url }, caption, gifPlayback };
    } else if (type === 'audio') {
      content = { audio: { url }, mimetype: 'audio/mp4', ptt: true };
    } else if (type === 'document') {
      content = { document: { url }, caption, fileName: caption };
    } else if (type === 'sticker') {
      content = { sticker: { url } };
    }

    const result = await sock.sendMessage(jid, content);

    return {
      success: true,
      id: result.key.id,
      timestamp: result.messageTimestamp,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send reaction (OpenClaw-compatible)
async function sendReaction({ chatJid, messageId, emoji, participant, fromMe = false, remove = false }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const jid = chatJid.includes('@') ? chatJid : `${chatJid}@s.whatsapp.net`;
    
    const key = {
      remoteJid: jid,
      id: messageId,
      fromMe,
      participant: participant || undefined,
    };

    const result = await sock.sendMessage(jid, {
      react: {
        text: remove ? '' : emoji,
        key,
      },
    });

    return {
      success: true,
      id: result.key.id,
      timestamp: result.messageTimestamp,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send poll (OpenClaw-compatible)
async function sendPoll({ to, name, options, selectableCount = 1 }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const jid = to.includes('@') ? to : `${to}@s.whatsapp.net`;

    const result = await sock.sendMessage(jid, {
      poll: {
        name,
        values: options.slice(0, 12), // Max 12 options
        selectableCount: Math.min(selectableCount, options.length),
      },
    });

    return {
      success: true,
      id: result.key.id,
      timestamp: result.messageTimestamp,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get contacts
async function getContacts() {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const contacts = await sock.store?.contacts || {};
    return { success: true, contacts: Object.values(contacts) };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get chats
async function getChats() {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const chats = await sock.store?.chats || {};
    return { success: true, chats: Object.values(chats) };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get profile picture
async function getProfilePicture({ jid }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const targetJid = jid.includes('@') ? jid : `${jid}@s.whatsapp.net`;
    const url = await sock.profilePictureUrl(targetJid, 'image');
    return { success: true, url };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get group metadata
async function getGroupMetadata({ groupJid }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const metadata = await sock.groupMetadata(groupJid);
    return { 
      success: true, 
      metadata: {
        id: metadata.id,
        subject: metadata.subject,
        participants: metadata.participants.map(p => ({
          id: p.id,
          admin: p.admin,
        })),
        owner: metadata.owner,
        creation: metadata.creation,
        desc: metadata.desc,
      }
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Mark as read
async function markAsRead({ chatJid, messageIds }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const jid = chatJid.includes('@') ? chatJid : `${chatJid}@s.whatsapp.net`;
    const keys = messageIds.map(id => ({
      remoteJid: jid,
      id,
      fromMe: false,
    }));
    await sock.readMessages(keys);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send typing indicator
async function sendPresenceUpdate({ chatJid, type = 'composing' }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const jid = chatJid.includes('@') ? chatJid : `${chatJid}@s.whatsapp.net`;
    await sock.sendPresenceUpdate(type, jid);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Logout
async function logout() {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    await sock.logout();
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (sock) {
    sock.end();
    sock = null;
  }
  return { success: true };
}

// Status
function status() {
  return {
    connected: sock !== null,
    user: sock?.user,
  };
}

// Handle JSON-RPC request
async function handleRequest(request) {
  const { id, method, params = {} } = request;

  try {
    let result;

    switch (method) {
      case 'init':
        result = await init();
        break;
      case 'send':
        result = await send(params);
        break;
      case 'sendMedia':
        result = await sendMedia(params);
        break;
      case 'sendReaction':
      case 'react':
        result = await sendReaction(params);
        break;
      case 'sendPoll':
      case 'poll':
        result = await sendPoll(params);
        break;
      case 'getContacts':
        result = await getContacts();
        break;
      case 'getChats':
        result = await getChats();
        break;
      case 'getProfilePicture':
        result = await getProfilePicture(params);
        break;
      case 'getGroupMetadata':
        result = await getGroupMetadata(params);
        break;
      case 'markAsRead':
        result = await markAsRead(params);
        break;
      case 'sendPresenceUpdate':
      case 'typing':
        result = await sendPresenceUpdate(params);
        break;
      case 'logout':
        result = await logout();
        break;
      case 'shutdown':
        result = await shutdown();
        setTimeout(() => process.exit(0), 100);
        break;
      case 'status':
        result = status();
        break;
      default:
        sendResponse(id, null, { code: -32601, message: `Unknown method: ${method}` });
        return;
    }

    sendResponse(id, result);
  } catch (error) {
    sendResponse(id, null, { code: -32603, message: error.message });
  }
}

// Main loop - read from stdin
const rl = createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

rl.on('line', async (line) => {
  if (!line.trim()) return;

  try {
    const request = JSON.parse(line);
    await handleRequest(request);
  } catch (error) {
    console.error('Parse error:', error.message);
  }
});

rl.on('close', () => {
  shutdown();
  process.exit(0);
});

// Handle process signals
process.on('SIGINT', () => {
  shutdown();
  process.exit(0);
});

process.on('SIGTERM', () => {
  shutdown();
  process.exit(0);
});

// Send ready event
sendEvent('ready', { bridge: 'whatsapp', version: '1.0.0' });
