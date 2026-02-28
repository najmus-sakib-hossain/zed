// Discord Bridge Runner
// Uses discord.js for Discord API
//
// JSON-RPC over stdin/stdout communication with Rust gateway

import { createInterface } from 'readline';
import {
  Client,
  GatewayIntentBits,
  Partials,
  EmbedBuilder,
  ActionRowBuilder,
  ButtonBuilder,
  ButtonStyle,
} from 'discord.js';

// State
let client = null;

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

// Initialize Discord client
async function init({ token }) {
  try {
    if (!token) {
      return { success: false, error: 'Bot token required' };
    }

    client = new Client({
      intents: [
        GatewayIntentBits.Guilds,
        GatewayIntentBits.GuildMessages,
        GatewayIntentBits.DirectMessages,
        GatewayIntentBits.MessageContent,
        GatewayIntentBits.GuildMembers,
      ],
      partials: [Partials.Channel, Partials.Message],
    });

    // Handle ready event
    client.once('ready', () => {
      sendEvent('state', {
        state: 'connected',
        user: {
          id: client.user.id,
          username: client.user.username,
          discriminator: client.user.discriminator,
          tag: client.user.tag,
        },
      });
    });

    // Handle errors
    client.on('error', (error) => {
      sendEvent('error', { message: error.message });
    });

    // Handle disconnect
    client.on('disconnect', () => {
      sendEvent('state', { state: 'disconnected' });
    });

    // Handle messages
    client.on('messageCreate', (message) => {
      // Ignore bot messages
      if (message.author.bot) return;

      sendEvent('message', {
        id: message.id,
        channel_id: message.channel.id,
        channel_name: message.channel.name,
        channel_type: message.channel.type,
        guild_id: message.guild?.id,
        guild_name: message.guild?.name,
        from: {
          id: message.author.id,
          username: message.author.username,
          discriminator: message.author.discriminator,
          tag: message.author.tag,
          bot: message.author.bot,
        },
        text: message.content,
        timestamp: message.createdTimestamp,
        attachments: message.attachments.map((a) => ({
          id: a.id,
          name: a.name,
          url: a.url,
          size: a.size,
          content_type: a.contentType,
        })),
        embeds: message.embeds.length,
        reply_to: message.reference?.messageId,
      });
    });

    // Handle button interactions
    client.on('interactionCreate', async (interaction) => {
      if (interaction.isButton()) {
        sendEvent('button', {
          id: interaction.id,
          custom_id: interaction.customId,
          user: {
            id: interaction.user.id,
            username: interaction.user.username,
            tag: interaction.user.tag,
          },
          channel_id: interaction.channel.id,
          message_id: interaction.message.id,
        });

        // Acknowledge the interaction
        await interaction.deferUpdate().catch(() => {});
      }
    });

    // Handle message reactions
    client.on('messageReactionAdd', (reaction, user) => {
      sendEvent('reaction', {
        type: 'add',
        message_id: reaction.message.id,
        channel_id: reaction.message.channel.id,
        emoji: reaction.emoji.name,
        user: {
          id: user.id,
          username: user.username,
          tag: user.tag,
        },
      });
    });

    // Login
    await client.login(token);

    return {
      success: true,
      user: {
        id: client.user.id,
        username: client.user.username,
        tag: client.user.tag,
      },
    };
  } catch (error) {
    sendEvent('state', { state: 'error', error: error.message });
    return { success: false, error: error.message };
  }
}

// Send a text message
async function send({ to, message, options = {} }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(to);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid or non-text channel' };
    }

    const result = await channel.send({
      content: message,
      reply: options.reply_to ? { messageReference: options.reply_to } : undefined,
    });

    return {
      success: true,
      message_id: result.id,
      channel_id: result.channel.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send an embed
async function sendEmbed({ to, embed, options = {} }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(to);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid or non-text channel' };
    }

    const embedBuilder = new EmbedBuilder()
      .setTitle(embed.title)
      .setDescription(embed.description)
      .setColor(embed.color || 0x5865f2);

    if (embed.url) embedBuilder.setURL(embed.url);
    if (embed.thumbnail) embedBuilder.setThumbnail(embed.thumbnail);
    if (embed.image) embedBuilder.setImage(embed.image);
    if (embed.footer) embedBuilder.setFooter({ text: embed.footer });
    if (embed.fields) {
      for (const field of embed.fields) {
        embedBuilder.addFields({ name: field.name, value: field.value, inline: field.inline });
      }
    }

    const result = await channel.send({ embeds: [embedBuilder] });

    return {
      success: true,
      message_id: result.id,
      channel_id: result.channel.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Send a message with buttons
async function sendWithButtons({ to, message, buttons, options = {} }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(to);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid or non-text channel' };
    }

    const rows = buttons.map((row) => {
      const actionRow = new ActionRowBuilder();
      for (const btn of row) {
        const button = new ButtonBuilder()
          .setCustomId(btn.id)
          .setLabel(btn.label)
          .setStyle(btn.style === 'danger' ? ButtonStyle.Danger : ButtonStyle.Primary);
        if (btn.emoji) button.setEmoji(btn.emoji);
        actionRow.addComponents(button);
      }
      return actionRow;
    });

    const result = await channel.send({
      content: message,
      components: rows,
    });

    return {
      success: true,
      message_id: result.id,
      channel_id: result.channel.id,
    };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Edit a message
async function editMessage({ channel_id, message_id, content }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(channel_id);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid channel' };
    }

    const message = await channel.messages.fetch(message_id);
    await message.edit(content);

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Delete a message
async function deleteMessage({ channel_id, message_id }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(channel_id);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid channel' };
    }

    const message = await channel.messages.fetch(message_id);
    await message.delete();

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Add reaction
async function addReaction({ channel_id, message_id, emoji }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const channel = await client.channels.fetch(channel_id);
    if (!channel || !channel.isTextBased()) {
      return { success: false, error: 'Invalid channel' };
    }

    const message = await channel.messages.fetch(message_id);
    await message.react(emoji);

    return { success: true };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get guilds (servers)
async function getGuilds() {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const guilds = client.guilds.cache.map((guild) => ({
      id: guild.id,
      name: guild.name,
      member_count: guild.memberCount,
      icon: guild.iconURL(),
    }));

    return { success: true, guilds };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Get channels in a guild
async function getChannels({ guild_id }) {
  if (!client) {
    return { success: false, error: 'Client not initialized' };
  }

  try {
    const guild = await client.guilds.fetch(guild_id);
    const channels = guild.channels.cache
      .filter((c) => c.isTextBased())
      .map((c) => ({
        id: c.id,
        name: c.name,
        type: c.type,
        parent_id: c.parentId,
      }));

    return { success: true, channels };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

// Shutdown
async function shutdown() {
  if (client) {
    client.destroy();
    client = null;
  }
  return { success: true };
}

// Status
function status() {
  return {
    connected: client?.isReady() || false,
    user: client?.user
      ? {
          id: client.user.id,
          username: client.user.username,
          tag: client.user.tag,
        }
      : null,
    guilds: client?.guilds.cache.size || 0,
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
      case 'sendEmbed':
        result = await sendEmbed(params);
        break;
      case 'sendWithButtons':
        result = await sendWithButtons(params);
        break;
      case 'editMessage':
        result = await editMessage(params);
        break;
      case 'deleteMessage':
        result = await deleteMessage(params);
        break;
      case 'addReaction':
        result = await addReaction(params);
        break;
      case 'getGuilds':
        result = await getGuilds();
        break;
      case 'getChannels':
        result = await getChannels(params);
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
sendEvent('ready', { bridge: 'discord', version: '1.0.0' });
