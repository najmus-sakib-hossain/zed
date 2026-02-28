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
import { isWhatsAppGroupJid, normalizeWhatsAppTarget, normalizeWhatsAppToJid } from './normalize.js';

const FRAMED_IPC = process.env.DX_BRIDGE_FRAMED_IPC === '1';
const PROTOCOL_VERSION = 1;
const FRAME_TYPE_BINARY = 0x02;

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
  writePayload(response);
}

// Send event to stdout
function sendEvent(event, payload) {
  writePayload({ event, payload });
}

function computeChecksum(buffer) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < buffer.length; i += 1) {
    hash = Math.imul(hash, 0x01000193) >>> 0;
    hash ^= buffer[i];
  }
  return hash >>> 0;
}

function encodeFrame(payloadObj) {
  const payload = Buffer.from(JSON.stringify(payloadObj), 'utf8');
  const frame = Buffer.alloc(6 + payload.length + 4);
  frame.writeUInt8(PROTOCOL_VERSION, 0);
  frame.writeUInt8(FRAME_TYPE_BINARY, 1);
  frame.writeUInt32BE(payload.length, 2);
  payload.copy(frame, 6);
  frame.writeUInt32BE(computeChecksum(payload), 6 + payload.length);
  return frame;
}

function decodeFrames(chunk, state) {
  state.buffer = Buffer.concat([state.buffer, chunk]);
  const payloads = [];

  while (state.buffer.length >= 6) {
    const version = state.buffer.readUInt8(0);
    if (version !== PROTOCOL_VERSION) {
      state.buffer = Buffer.alloc(0);
      break;
    }

    const frameType = state.buffer.readUInt8(1);
    const payloadLen = state.buffer.readUInt32BE(2);
    const totalLen = 6 + payloadLen + 4;

    if (state.buffer.length < totalLen) {
      break;
    }

    const payload = state.buffer.subarray(6, 6 + payloadLen);
    const checksum = state.buffer.readUInt32BE(6 + payloadLen);
    state.buffer = state.buffer.subarray(totalLen);

    if (frameType !== FRAME_TYPE_BINARY || computeChecksum(payload) !== checksum) {
      continue;
    }

    try {
      payloads.push(JSON.parse(payload.toString('utf8')));
    } catch {
      // ignore malformed payloads
    }
  }

  return payloads;
}

function writePayload(payloadObj) {
  if (FRAMED_IPC) {
    const frame = encodeFrame(payloadObj);
    process.stdout.write(frame);
    return;
  }

  console.log(JSON.stringify(payloadObj));
}

function parseAllowFrom(allowFrom) {
  if (!Array.isArray(allowFrom)) {
    return { hasWildcard: false, allowList: [] };
  }

  const raw = allowFrom
    .map((entry) => String(entry).trim())
    .filter(Boolean);

  const hasWildcard = raw.includes('*');
  const allowList = raw
    .filter((entry) => entry !== '*')
    .map((entry) => normalizeWhatsAppTarget(entry))
    .filter((entry) => Boolean(entry));

  return { hasWildcard, allowList };
}

function resolveTarget({ to, allowFrom = [], mode = 'explicit' }) {
  const trimmed = String(to ?? '').trim();
  if (!trimmed) {
    return {
      ok: false,
      error: 'Missing WhatsApp target. Expected <E.164|group JID>.',
    };
  }

  const normalized = normalizeWhatsAppTarget(trimmed);
  if (!normalized) {
    return {
      ok: false,
      error: 'Invalid WhatsApp target. Expected <E.164|group JID>.',
    };
  }

  if (isWhatsAppGroupJid(normalized)) {
    return { ok: true, target: normalized, normalized };
  }

  const normalizedMode = String(mode).toLowerCase();
  if (normalizedMode === 'implicit' || normalizedMode === 'heartbeat') {
    const { hasWildcard, allowList } = parseAllowFrom(allowFrom);
    if (!hasWildcard && allowList.length > 0 && !allowList.includes(normalized)) {
      return {
        ok: false,
        error: 'WhatsApp target blocked by allow_from policy.',
      };
    }
  }

  const jid = normalizeWhatsAppToJid(normalized);
  if (!jid) {
    return {
      ok: false,
      error: 'Failed to convert normalized target to JID.',
    };
  }

  return {
    ok: true,
    target: jid,
    normalized,
  };
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
async function send({ to, message, options = {}, allowFrom = [], mode = 'explicit' }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const resolved = resolveTarget({ to, allowFrom, mode });
    if (!resolved.ok) {
      return { success: false, error: resolved.error };
    }
    const jid = resolved.target;

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
async function sendMedia({ to, url, caption, type = 'image', gifPlayback = false, allowFrom = [], mode = 'explicit' }) {
  if (!sock) {
    return { success: false, error: 'Not connected' };
  }

  try {
    const resolved = resolveTarget({ to, allowFrom, mode });
    if (!resolved.ok) {
      return { success: false, error: resolved.error };
    }
    const jid = resolved.target;

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
    const jid = normalizeWhatsAppToJid(chatJid);
    if (!jid) {
      return { success: false, error: `Invalid WhatsApp target: ${chatJid}` };
    }
    
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
    const resolved = resolveTarget({ to });
    if (!resolved.ok) {
      return { success: false, error: resolved.error };
    }
    const jid = resolved.target;

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
    const jid = normalizeWhatsAppToJid(chatJid);
    if (!jid) {
      return { success: false, error: `Invalid WhatsApp target: ${chatJid}` };
    }
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
    const jid = normalizeWhatsAppToJid(chatJid);
    if (!jid) {
      return { success: false, error: `Invalid WhatsApp target: ${chatJid}` };
    }
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
      case 'resolveTarget':
        result = resolveTarget(params);
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

if (FRAMED_IPC) {
  const state = { buffer: Buffer.alloc(0) };

  process.stdin.on('data', async (chunk) => {
    const requests = decodeFrames(chunk, state);
    for (const request of requests) {
      await handleRequest(request);
    }
  });

  process.stdin.on('close', () => {
    shutdown();
    process.exit(0);
  });
} else {
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
}

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
