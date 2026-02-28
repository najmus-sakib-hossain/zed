#!/usr/bin/env bun
// Telegram bridge using telegraf

import { Telegraf } from 'telegraf';

let bot;

process.stdin.on('data', async (data) => {
    try {
        const config = JSON.parse(data.toString());
        
        if (config.token && !bot) {
            bot = new Telegraf(config.token);
            
            bot.on('message', (ctx) => {
                const msg = {
                    type: 'message',
                    id: ctx.message.message_id,
                    from: ctx.message.from.id,
                    username: ctx.message.from.username,
                    text: ctx.message.text,
                    chat_id: ctx.message.chat.id,
                    timestamp: ctx.message.date
                };
                console.log('MESSAGE:', JSON.stringify(msg));
            });
            
            bot.launch();
            console.log('READY');
        } else if (config.action === 'send') {
            await bot.telegram.sendMessage(config.chat_id, config.message);
            console.log('SENT:', config.chat_id);
        }
    } catch (err) {
        console.error('ERROR:', err.message);
    }
});

process.once('SIGINT', () => bot.stop('SIGINT'));
process.once('SIGTERM', () => bot.stop('SIGTERM'));
