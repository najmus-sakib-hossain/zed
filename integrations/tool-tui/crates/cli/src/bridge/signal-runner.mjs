// Signal Bridge Runner
// Uses signal-cli for Signal protocol
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import { spawn } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

// State
let signalProcess = null;
let phoneNumber = null;

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

// Check if signal-cli is available
function findSignalCli() {
  const paths = [
    '/usr/local/bin/signal-cli',
    '/usr/bin/signal-cli',
    join(homedir(), '.local', 'bin', 'signal-cli'),
    'signal-cli', // In PATH
  ];

  for (const p of paths) {
    try {
      if (existsSync(p)) return p;
    } catch {}
  }
  return 'signal-cli'; // Hope it's in PATH
}

// Initialize Signal connection
async function init({ phone, configPath }) {
  try {
    if (!phone) {
      return { success: false, error: 'Phone number required (+1234567890)' };
    }

    phoneNumber = phone;
    const signalCliPath = findSignalCli();
    const config = configPath || join(homedir(), '.local', 'share', 'signal-cli');

    // Start signal-cli in daemon mode with JSON output
    signalProcess = spawn(signalCliPath, [
      '-u', phone,
      '--config', config,
      'daemon',
      '--json',
    ], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    // Handle stdout (JSON messages)
    const rl = createInterface({
      input: signalProcess.stdout,
      crlfDelay: Infinity,
    });

    rl.on('line', (line) => {
      if (!line.trim()) return;

      try {
        const data = JSON.parse(line);

        if (data.envelope) {
          const env = data.envelope;

          if (env.dataMessage) {
            sendEvent('message', {
              timestamp: env.timestamp,
              from: env.source,
              to: phoneNumber,
              group_id: env.dataMessage.groupInfo?.groupId,
              text: env.dataMessage.message,
              attachments: env.dataMessage.attachments?.map(a => ({
                id: a.id,
                contentType: a.contentType,
                filename: a.filename,
                size: a.size,
              })),
              quote: env.dataMessage.quote,
              reaction: env.dataMessage.reaction,
            });
          }

          if (env.receiptMessage) {
            sendEvent('receipt', {
              timestamp: env.timestamp,
              from: env.source,
              type: env.receiptMessage.isDelivery ? 'delivery' : 'read',
              timestamps: env.receiptMessage.timestamps,
            });
          }

          if (env.typingMessage) {
            sendEvent('typing', {
              from: env.source,
              group_id: env.typingMessage.groupId,
              started: env.typingMessage.action === 'STARTED',
            });
          }
        }
      } catch (e) {
        // Not JSON, might be status message
      }
    });

    // Handle stderr
    signalProcess.stderr.on('data', (data) => {
      const msg = data.toString().trim();
      if (msg) {
        console.error('signal-cli:', msg);
      }
    });

    // Handle process exit
    signalProcess.on('close', (code) => {
      sendEvent('state', { state: 'disconnected', code });
      signalProcess = null;
    });

    signalProcess.on('error', (error) => {
      sendEvent('state', { state: 'error', error: error.message });
    });

    // Wait a bit for startup
    await new Promise(resolve => setTimeout(resolve, 1000));

    sendEvent('state', { state: 'connected' });

    return {
      success: true,
      phone: phoneNumber,
    };
  } catch (error) {
    sendEvent('state', { state: 'error', error: error.message });
    return { success: false, error: error.message };
  }
}

// Send a command to signal-cli daemon
function sendCommand(command) {
  return new Promise((resolve, reject) => {
    if (!signalProcess) {
      reject(new Error('Signal not connected'));
      return;
    }

    const timeout = setTimeout(() => {
      reject(new Error('Command timeout'));
    }, 30000);

    // For daemon mode, we need to send JSON-RPC commands
    const request = JSON.stringify(command) + '\n';
    signalProcess.stdin.write(request);

    // The daemon doesn't have request-response, so we just assume success
    clearTimeout(timeout);
    resolve({ success: true });
  });
}

// Send a text message
async function send({ to, message, options = {} }) {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    // Use signal-cli send command via daemon's stdin
    const command = {
      jsonrpc: '2.0',
      method: 'send',
      params: {
        recipient: [to],
        message,
        ...(options.quote && { quoteTimestamp: options.quote }),
      },
      id: Date.now(),
    };

    await sendCommand(command);

    return {
      success: true,
      timestamp: Date.now(),
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send to a group
async function sendGroup({ groupId, message, options = {} }) {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    const command = {
      jsonrpc: '2.0',
      method: 'send',
      params: {
        groupId,
        message,
      },
      id: Date.now(),
    };

    await sendCommand(command);

    return {
      success: true,
      timestamp: Date.now(),
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a reaction
async function sendReaction({ to, targetTimestamp, emoji, remove = false }) {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    const command = {
      jsonrpc: '2.0',
      method: 'sendReaction',
      params: {
        recipient: [to],
        emoji,
        targetAuthor: to,
        targetTimestamp,
        remove,
      },
      id: Date.now(),
    };

    await sendCommand(command);

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get contacts
async function getContacts() {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    const command = {
      jsonrpc: '2.0',
      method: 'listContacts',
      id: Date.now(),
    };

    await sendCommand(command);

    // Note: Actual contacts come via events
    return { success: true, message: 'Contacts will be received via events' };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get groups
async function getGroups() {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    const command = {
      jsonrpc: '2.0',
      method: 'listGroups',
      id: Date.now(),
    };

    await sendCommand(command);

    return { success: true, message: 'Groups will be received via events' };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Trust identity (verify safety number)
async function trustIdentity({ recipient, trustAllKnown = false }) {
  if (!signalProcess) {
    return { success: false, error: 'Signal not connected' };
  }

  try {
    const command = {
      jsonrpc: '2.0',
      method: 'trust',
      params: {
        recipient: [recipient],
        trustAllKnownKeys: trustAllKnown,
      },
      id: Date.now(),
    };

    await sendCommand(command);

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (signalProcess) {
    signalProcess.kill();
    signalProcess = null;
  }
  return { success: true };
}

// Status
function status() {
  return {
    connected: signalProcess !== null,
    phone: phoneNumber,
  };
}

// Handle JSON-RPC request
async function handleRequest(request) {
  const { id, method, params = {} } = request;

  try {
    let result;

    switch (method) {
      case 'init':
        result = await init(params);
        break;
      case 'send':
        result = await send(params);
        break;
      case 'sendGroup':
        result = await sendGroup(params);
        break;
      case 'sendReaction':
        result = await sendReaction(params);
        break;
      case 'getContacts':
        result = await getContacts();
        break;
      case 'getGroups':
        result = await getGroups();
        break;
      case 'trustIdentity':
        result = await trustIdentity(params);
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
sendEvent('ready', { bridge: 'signal', version: '1.0.0' });
