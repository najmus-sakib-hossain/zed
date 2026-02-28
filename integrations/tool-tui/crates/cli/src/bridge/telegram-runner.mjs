// Telegram Bridge Runner
// Uses grammy for Telegram Bot API
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import { Bot, GrammyError, HttpError } from 'grammy';

// State
let bot = null;
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

// Initialize Telegram bot
async function init({ token }) {
  try {
    if (!token) {
      return { success: false, error: 'Bot token required' };
    }

    bot = new Bot(token);

    // Handle errors
    bot.catch((err) => {
      const ctx = err.ctx;
      console.error(`Error while handling update ${ctx.update.update_id}:`);
      const e = err.error;
      if (e instanceof GrammyError) {
        console.error('Error in request:', e.description);
      } else if (e instanceof HttpError) {
        console.error('Could not contact Telegram:', e);
      } else {
        console.error('Unknown error:', e);
      }
    });

    // Handle text messages
    bot.on('message:text', (ctx) => {
      sendEvent('message', {
        id: ctx.message.message_id,
        chat_id: ctx.chat.id,
        chat_type: ctx.chat.type,
        from: {
          id: ctx.from.id,
          username: ctx.from.username,
          first_name: ctx.from.first_name,
          last_name: ctx.from.last_name,
        },
        text: ctx.message.text,
        timestamp: ctx.message.date,
        reply_to: ctx.message.reply_to_message?.message_id,
      });
    });

    // Handle photo messages
    bot.on('message:photo', (ctx) => {
      const photo = ctx.message.photo[ctx.message.photo.length - 1];
      sendEvent('message', {
        id: ctx.message.message_id,
        chat_id: ctx.chat.id,
        from: {
          id: ctx.from.id,
          username: ctx.from.username,
        },
        type: 'photo',
        file_id: photo.file_id,
        caption: ctx.message.caption,
        timestamp: ctx.message.date,
      });
    });

    // Handle document messages
    bot.on('message:document', (ctx) => {
      sendEvent('message', {
        id: ctx.message.message_id,
        chat_id: ctx.chat.id,
        from: {
          id: ctx.from.id,
          username: ctx.from.username,
        },
        type: 'document',
        file_id: ctx.message.document.file_id,
        file_name: ctx.message.document.file_name,
        mime_type: ctx.message.document.mime_type,
        caption: ctx.message.caption,
        timestamp: ctx.message.date,
      });
    });

    // Handle voice messages
    bot.on('message:voice', (ctx) => {
      sendEvent('message', {
        id: ctx.message.message_id,
        chat_id: ctx.chat.id,
        from: {
          id: ctx.from.id,
          username: ctx.from.username,
        },
        type: 'voice',
        file_id: ctx.message.voice.file_id,
        duration: ctx.message.voice.duration,
        timestamp: ctx.message.date,
      });
    });

    // Handle callback queries (button clicks)
    bot.on('callback_query:data', (ctx) => {
      sendEvent('callback', {
        id: ctx.callbackQuery.id,
        chat_id: ctx.chat?.id,
        from: {
          id: ctx.from.id,
          username: ctx.from.username,
        },
        data: ctx.callbackQuery.data,
        message_id: ctx.callbackQuery.message?.message_id,
      });
      ctx.answerCallbackQuery();
    });

    // Start bot
    botInfo = await bot.api.getMe();
    bot.start();

    sendEvent('state', { state: 'connected' });

    return {
      success: true,
      bot: {
        id: botInfo.id,
        username: botInfo.username,
        first_name: botInfo.first_name,
      },
    };
  } catch (error) {
    sendEvent('state', { state: 'error', error: error.message });
    return { success: false, error: error.message };
  }
}

// Send a text message
async function send({ to, message, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.sendMessage(to, message, {
      parse_mode: options.parse_mode || 'Markdown',
      reply_to_message_id: options.reply_to,
      disable_notification: options.silent,
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a photo
async function sendPhoto({ to, url, caption, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.sendPhoto(to, url, {
      caption,
      parse_mode: options.parse_mode || 'Markdown',
      reply_to_message_id: options.reply_to,
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a document
async function sendDocument({ to, url, caption, filename, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.sendDocument(to, url, {
      caption,
      parse_mode: options.parse_mode || 'Markdown',
      reply_to_message_id: options.reply_to,
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a message with inline keyboard
async function sendWithButtons({ to, message, buttons, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const inline_keyboard = buttons.map((row) =>
      row.map((btn) => ({
        text: btn.text,
        callback_data: btn.data,
        url: btn.url,
      }))
    );

    const result = await bot.api.sendMessage(to, message, {
      parse_mode: options.parse_mode || 'Markdown',
      reply_markup: { inline_keyboard },
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Edit a message
async function editMessage({ chat_id, message_id, text, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.editMessageText(chat_id, message_id, text, {
      parse_mode: options.parse_mode || 'Markdown',
    });

    return { success: true, result };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Delete a message
async function deleteMessage({ chat_id, message_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    await bot.api.deleteMessage(chat_id, message_id);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get chat info
async function getChat({ chat_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const chat = await bot.api.getChat(chat_id);
    return { success: true, chat };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get file URL
async function getFileUrl({ file_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const file = await bot.api.getFile(file_id);
    const url = `https://api.telegram.org/file/bot${bot.token}/${file.file_path}`;
    return { success: true, url, file };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Set reaction on a message (OpenClaw-compatible)
async function setReaction({ chat_id, message_id, emoji, is_big = false }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const reaction = emoji ? [{ type: 'emoji', emoji }] : [];
    await bot.api.setMessageReaction(chat_id, message_id, {
      reaction,
      is_big,
    });
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send poll (OpenClaw-compatible)
async function sendPoll({ to, question, options, is_anonymous = true, allows_multiple_answers = false, type = 'regular' }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.sendPoll(to, question, options, {
      is_anonymous,
      allows_multiple_answers,
      type,
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
      poll: result.poll,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send chat action (typing, uploading, etc.)
async function sendChatAction({ chat_id, action = 'typing' }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    await bot.api.sendChatAction(chat_id, action);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Pin message
async function pinMessage({ chat_id, message_id, disable_notification = false }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    await bot.api.pinChatMessage(chat_id, message_id, { disable_notification });
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Unpin message
async function unpinMessage({ chat_id, message_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    await bot.api.unpinChatMessage(chat_id, message_id);
    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get chat member info
async function getChatMember({ chat_id, user_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const member = await bot.api.getChatMember(chat_id, user_id);
    return { success: true, member };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send voice message
async function sendVoice({ to, url, caption, options = {} }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.sendVoice(to, url, {
      caption,
      parse_mode: options.parse_mode || 'Markdown',
      reply_to_message_id: options.reply_to,
    });

    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Forward message
async function forwardMessage({ chat_id, from_chat_id, message_id }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.forwardMessage(chat_id, from_chat_id, message_id);
    return {
      success: true,
      message_id: result.message_id,
      chat_id: result.chat.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Copy message
async function copyMessage({ chat_id, from_chat_id, message_id, caption }) {
  if (!bot) {
    return { success: false, error: 'Bot not initialized' };
  }

  try {
    const result = await bot.api.copyMessage(chat_id, from_chat_id, message_id, { caption });
    return {
      success: true,
      message_id: result.message_id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (bot) {
    await bot.stop();
    bot = null;
  }
  return { success: true };
}

// Status
function status() {
  return {
    connected: bot !== null,
    bot: botInfo
      ? {
          id: botInfo.id,
          username: botInfo.username,
          first_name: botInfo.first_name,
        }
      : null,
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
      case 'sendPhoto':
        result = await sendPhoto(params);
        break;
      case 'sendDocument':
        result = await sendDocument(params);
        break;
      case 'sendVoice':
        result = await sendVoice(params);
        break;
      case 'sendWithButtons':
        result = await sendWithButtons(params);
        break;
      case 'sendPoll':
      case 'poll':
        result = await sendPoll(params);
        break;
      case 'editMessage':
        result = await editMessage(params);
        break;
      case 'deleteMessage':
        result = await deleteMessage(params);
        break;
      case 'getChat':
        result = await getChat(params);
        break;
      case 'getFileUrl':
        result = await getFileUrl(params);
        break;
      case 'setReaction':
      case 'react':
        result = await setReaction(params);
        break;
      case 'sendChatAction':
      case 'typing':
        result = await sendChatAction(params);
        break;
      case 'pinMessage':
        result = await pinMessage(params);
        break;
      case 'unpinMessage':
        result = await unpinMessage(params);
        break;
      case 'getChatMember':
        result = await getChatMember(params);
        break;
      case 'forwardMessage':
        result = await forwardMessage(params);
        break;
      case 'copyMessage':
        result = await copyMessage(params);
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
sendEvent('ready', { bridge: 'telegram', version: '1.0.0' });
