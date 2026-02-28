#!/usr/bin/env bun
// Discord bridge using discord.js

import { Client, GatewayIntentBits } from 'discord.js';

let client;

process.stdin.on('data', async (data) => {
    try {
        const config = JSON.parse(data.toString());
        
        if (config.token && !client) {
            client = new Client({
                intents: [
                    GatewayIntentBits.Guilds,
                    GatewayIntentBits.GuildMessages,
                    GatewayIntentBits.MessageContent,
                    GatewayIntentBits.DirectMessages
                ]
            });
            
            client.on('ready', () => {
                console.log('READY');
            });
            
            client.on('messageCreate', (message) => {
                if (message.author.bot) return;
                
                const msg = {
                    type: 'message',
                    id: message.id,
                    from: message.author.id,
                    username: message.author.username,
                    content: message.content,
                    channel_id: message.channelId,
                    guild_id: message.guildId,
                    timestamp: message.createdTimestamp
                };
                console.log('MESSAGE:', JSON.stringify(msg));
            });
            
            await client.login(config.token);
        } else if (config.action === 'send') {
            const channel = await client.channels.fetch(config.channel_id);
            await channel.send(config.message);
            console.log('SENT:', config.channel_id);
        }
    } catch (err) {
        console.error('ERROR:', err.message);
    }
});

process.on('SIGINT', () => client.destroy());
process.on('SIGTERM', () => client.destroy());
