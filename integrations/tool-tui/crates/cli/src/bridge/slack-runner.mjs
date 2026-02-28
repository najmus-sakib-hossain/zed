// Slack Bridge Runner
// Uses @slack/web-api and @slack/bolt
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import { WebClient } from '@slack/web-api';
import { App } from '@slack/bolt';

// State
let webClient = null;
let boltApp = null;
let botInfo = null;

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

// Initialize Slack client
async function init({ token, appToken, signingSecret }) {
  try {
    if (!token) {
      return { success: false, error: 'Bot token required' };
    }

    webClient = new WebClient(token);

    // Get bot info
    const authTest = await webClient.auth.test();
    botInfo = {
      user_id: authTest.user_id,
      bot_id: authTest.bot_id,
      team_id: authTest.team_id,
      team: authTest.team,
      user: authTest.user,
    };

    // If app token provided, set up Socket Mode for events
    if (appToken && signingSecret) {
      boltApp = new App({
        token,
        appToken,
        signingSecret,
        socketMode: true,
      });

      // Handle messages
      boltApp.message(async ({ message, say, client }) => {
        if (message.subtype) return; // Ignore edits, deletions, etc.

        sendEvent('message', {
          id: message.ts,
          channel_id: message.channel,
          thread_ts: message.thread_ts,
          from: {
            id: message.user,
          },
          text: message.text,
          timestamp: parseFloat(message.ts),
          blocks: message.blocks,
          attachments: message.attachments,
        });
      });

      // Handle mentions
      boltApp.event('app_mention', async ({ event, say }) => {
        sendEvent('mention', {
          id: event.ts,
          channel_id: event.channel,
          thread_ts: event.thread_ts,
          from: {
            id: event.user,
          },
          text: event.text,
          timestamp: parseFloat(event.ts),
        });
      });

      // Handle reactions
      boltApp.event('reaction_added', async ({ event }) => {
        sendEvent('reaction', {
          type: 'add',
          reaction: event.reaction,
          user: event.user,
          item: event.item,
        });
      });

      // Handle button clicks
      boltApp.action(/.*/, async ({ action, ack, body }) => {
        await ack();
        sendEvent('action', {
          action_id: action.action_id,
          value: action.value || action.selected_option?.value,
          user: body.user.id,
          channel: body.channel?.id,
          message_ts: body.message?.ts,
        });
      });

      await boltApp.start();
      sendEvent('state', { state: 'connected', mode: 'socket' });
    } else {
      sendEvent('state', { state: 'connected', mode: 'api-only' });
    }

    return {
      success: true,
      bot: botInfo,
    };
  } catch (error) {
    sendEvent('state', { state: 'error', error: error.message });
    return { success: false, error: error.message };
  }
}

// Send a text message
async function send({ to, message, options = {} }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.chat.postMessage({
      channel: to,
      text: message,
      thread_ts: options.thread_ts,
      mrkdwn: true,
    });

    return {
      success: true,
      ts: result.ts,
      channel: result.channel,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a message with blocks (rich formatting)
async function sendBlocks({ to, text, blocks, options = {} }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.chat.postMessage({
      channel: to,
      text, // Fallback text
      blocks,
      thread_ts: options.thread_ts,
    });

    return {
      success: true,
      ts: result.ts,
      channel: result.channel,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a message with attachments
async function sendAttachment({ to, text, attachments, options = {} }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.chat.postMessage({
      channel: to,
      text,
      attachments,
      thread_ts: options.thread_ts,
    });

    return {
      success: true,
      ts: result.ts,
      channel: result.channel,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Update a message
async function updateMessage({ channel, ts, text, blocks }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.chat.update({
      channel,
      ts,
      text,
      blocks,
    });

    return { success: true, ts: result.ts };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Delete a message
async function deleteMessage({ channel, ts }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    await webClient.chat.delete({
      channel,
      ts,
    });

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Add reaction
async function addReaction({ channel, ts, emoji }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    await webClient.reactions.add({
      channel,
      timestamp: ts,
      name: emoji,
    });

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Upload a file
async function uploadFile({ channel, content, filename, title, initial_comment }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.files.uploadV2({
      channel_id: channel,
      content,
      filename,
      title,
      initial_comment,
    });

    return {
      success: true,
      file: result.file,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get channels
async function getChannels({ types = 'public_channel,private_channel' }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.conversations.list({
      types,
      limit: 1000,
    });

    const channels = result.channels.map((c) => ({
      id: c.id,
      name: c.name,
      is_private: c.is_private,
      is_member: c.is_member,
      topic: c.topic?.value,
      purpose: c.purpose?.value,
    }));

    return { success: true, channels };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get users
async function getUsers() {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.users.list({ limit: 1000 });

    const users = result.members
      .filter((m) => !m.deleted && !m.is_bot)
      .map((m) => ({
        id: m.id,
        name: m.name,
        real_name: m.real_name,
        display_name: m.profile?.display_name,
        email: m.profile?.email,
        avatar: m.profile?.image_72,
      }));

    return { success: true, users };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get user info
async function getUserInfo({ user_id }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.users.info({ user: user_id });

    return {
      success: true,
      user: {
        id: result.user.id,
        name: result.user.name,
        real_name: result.user.real_name,
        display_name: result.user.profile?.display_name,
        email: result.user.profile?.email,
        avatar: result.user.profile?.image_72,
      },
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get conversation history
async function getHistory({ channel, limit = 100 }) {
  if (!webClient) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const result = await webClient.conversations.history({
      channel,
      limit,
    });

    const messages = result.messages.map((m) => ({
      ts: m.ts,
      user: m.user,
      text: m.text,
      thread_ts: m.thread_ts,
      reply_count: m.reply_count,
    }));

    return { success: true, messages };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (boltApp) {
    await boltApp.stop();
    boltApp = null;
  }
  webClient = null;
  return { success: true };
}

// Status
function status() {
  return {
    connected: webClient !== null,
    bot: botInfo,
    socketMode: boltApp !== null,
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
      case 'sendBlocks':
        result = await sendBlocks(params);
        break;
      case 'sendAttachment':
        result = await sendAttachment(params);
        break;
      case 'updateMessage':
        result = await updateMessage(params);
        break;
      case 'deleteMessage':
        result = await deleteMessage(params);
        break;
      case 'addReaction':
        result = await addReaction(params);
        break;
      case 'uploadFile':
        result = await uploadFile(params);
        break;
      case 'getChannels':
        result = await getChannels(params);
        break;
      case 'getUsers':
        result = await getUsers();
        break;
      case 'getUserInfo':
        result = await getUserInfo(params);
        break;
      case 'getHistory':
        result = await getHistory(params);
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
sendEvent('ready', { bridge: 'slack', version: '1.0.0' });
