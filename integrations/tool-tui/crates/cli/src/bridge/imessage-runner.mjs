// iMessage Bridge Runner
// Uses AppleScript on macOS for iMessage integration
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import { execFile } from 'child_process';
import { promisify } from 'util';
import { platform } from 'os';

const execFileAsync = promisify(execFile);

// State
let initialized = false;
let messageCheckInterval = null;
let lastMessageId = null;

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

// Execute AppleScript
async function runAppleScript(script) {
  if (platform() !== 'darwin') {
    throw new Error('iMessage is only available on macOS');
  }

  try {
    const { stdout } = await execFileAsync('osascript', ['-e', script]);
    return stdout.trim();
  } catch (error) {
    throw new Error(`AppleScript error: ${error.message}`);
  }
}

// Initialize iMessage connection
async function init() {
  try {
    if (platform() !== 'darwin') {
      return { success: false, error: 'iMessage is only available on macOS' };
    }

    // Check if Messages app is available
    const checkScript = `
      tell application "System Events"
        return exists application process "Messages"
      end tell
    `;

    await runAppleScript(checkScript);
    initialized = true;

    // Start polling for new messages
    startMessagePolling();

    sendEvent('state', { state: 'connected' });

    return { success: true };
  } catch (error) {
    sendEvent('state', { state: 'error', error: error.message });
    return { success: false, error: error.message };
  }
}

// Poll for new messages
function startMessagePolling() {
  if (messageCheckInterval) {
    clearInterval(messageCheckInterval);
  }

  messageCheckInterval = setInterval(async () => {
    try {
      await checkNewMessages();
    } catch (error) {
      console.error('Message poll error:', error.message);
    }
  }, 2000); // Check every 2 seconds
}

// Check for new messages using Messages.app
async function checkNewMessages() {
  if (!initialized) return;

  const script = `
    tell application "Messages"
      set allChats to every chat
      set results to ""
      repeat with aChat in allChats
        set chatId to id of aChat
        set msgs to messages of aChat
        if (count of msgs) > 0 then
          set lastMsg to item 1 of msgs
          set msgId to id of lastMsg
          set msgText to text of lastMsg
          set msgDate to date received of lastMsg
          set msgSender to handle of sender of lastMsg
          set results to results & chatId & "|||" & msgId & "|||" & msgSender & "|||" & msgText & "|||" & (msgDate as string) & "~~~"
        end if
      end repeat
      return results
    end tell
  `;

  try {
    const result = await runAppleScript(script);

    if (!result) return;

    const messages = result.split('~~~').filter(m => m.trim());

    for (const msg of messages) {
      const [chatId, msgId, sender, text, date] = msg.split('|||');

      if (msgId && msgId !== lastMessageId) {
        // New message
        sendEvent('message', {
          id: msgId,
          chat_id: chatId,
          from: sender,
          text: text || '',
          timestamp: date,
        });

        lastMessageId = msgId;
      }
    }
  } catch (error) {
    // Messages app might not have permission or be closed
  }
}

// Send a text message
async function send({ to, message }) {
  if (!initialized) {
    return { success: false, error: 'iMessage not initialized' };
  }

  try {
    // Escape special characters for AppleScript
    const escapedMessage = message
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"')
      .replace(/\n/g, '\\n');

    const script = `
      tell application "Messages"
        set targetBuddy to "${to}"
        set targetService to 1st account whose service type = iMessage
        set theBuddy to participant targetBuddy of targetService
        send "${escapedMessage}" to theBuddy
      end tell
    `;

    await runAppleScript(script);

    return {
      success: true,
      timestamp: Date.now(),
    };
  } catch (error) {
    // Try alternate method using buddy by phone/email
    try {
      const altScript = `
        tell application "Messages"
          send "${message.replace(/"/g, '\\"')}" to buddy "${to}" of (service 1 whose service type is iMessage)
        end tell
      `;
      await runAppleScript(altScript);

      return {
        success: true,
        timestamp: Date.now(),
      };
    } catch (altError) {
      return { success: false, error: error.message };
    }
  }
}

// Send to a group chat (by chat ID)
async function sendGroup({ chatId, message }) {
  if (!initialized) {
    return { success: false, error: 'iMessage not initialized' };
  }

  try {
    const escapedMessage = message
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"');

    const script = `
      tell application "Messages"
        set theChat to chat id "${chatId}"
        send "${escapedMessage}" to theChat
      end tell
    `;

    await runAppleScript(script);

    return {
      success: true,
      timestamp: Date.now(),
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get recent chats
async function getChats() {
  if (!initialized) {
    return { success: false, error: 'iMessage not initialized' };
  }

  try {
    const script = `
      tell application "Messages"
        set allChats to every chat
        set results to ""
        repeat with aChat in allChats
          set chatId to id of aChat
          set chatName to name of aChat
          set participants to ""
          repeat with p in participants of aChat
            set participants to participants & (handle of p) & ","
          end repeat
          set results to results & chatId & "|||" & chatName & "|||" & participants & "~~~"
        end repeat
        return results
      end tell
    `;

    const result = await runAppleScript(script);
    const chats = [];

    if (result) {
      const chatStrings = result.split('~~~').filter(c => c.trim());

      for (const chatStr of chatStrings) {
        const [id, name, participantsStr] = chatStr.split('|||');
        const participants = participantsStr ? participantsStr.split(',').filter(p => p) : [];

        chats.push({
          id,
          name: name || 'Unknown',
          participants,
        });
      }
    }

    return { success: true, chats };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get contacts (from Contacts.app)
async function getContacts() {
  try {
    const script = `
      tell application "Contacts"
        set results to ""
        repeat with p in every person
          set pName to name of p
          set pPhones to ""
          repeat with ph in phones of p
            set pPhones to pPhones & (value of ph) & ","
          end repeat
          set pEmails to ""
          repeat with em in emails of p
            set pEmails to pEmails & (value of em) & ","
          end repeat
          set results to results & pName & "|||" & pPhones & "|||" & pEmails & "~~~"
        end repeat
        return results
      end tell
    `;

    const result = await runAppleScript(script);
    const contacts = [];

    if (result) {
      const contactStrings = result.split('~~~').filter(c => c.trim());

      for (const contactStr of contactStrings) {
        const [name, phonesStr, emailsStr] = contactStr.split('|||');
        const phones = phonesStr ? phonesStr.split(',').filter(p => p) : [];
        const emails = emailsStr ? emailsStr.split(',').filter(e => e) : [];

        contacts.push({
          name: name || 'Unknown',
          phones,
          emails,
        });
      }
    }

    return { success: true, contacts };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Open Messages app
async function openMessages() {
  try {
    await runAppleScript('tell application "Messages" to activate');
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (messageCheckInterval) {
    clearInterval(messageCheckInterval);
    messageCheckInterval = null;
  }
  initialized = false;
  return { success: true };
}

// Status
function status() {
  return {
    connected: initialized,
    platform: platform(),
    available: platform() === 'darwin',
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
      case 'sendGroup':
        result = await sendGroup(params);
        break;
      case 'getChats':
        result = await getChats();
        break;
      case 'getContacts':
        result = await getContacts();
        break;
      case 'openMessages':
        result = await openMessages();
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
sendEvent('ready', {
  bridge: 'imessage',
  version: '1.0.0',
  available: platform() === 'darwin',
});
