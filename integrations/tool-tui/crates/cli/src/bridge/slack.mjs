#!/usr/bin/env bun
// Slack bridge using @slack/bolt

import { App } from '@slack/bolt';

let app;

process.stdin.on('data', async (data) => {
    try {
        const config = JSON.parse(data.toString());
        
        if (config.token && config.signing_secret && !app) {
            app = new App({
                token: config.token,
                signingSecret: config.signing_secret,
                socketMode: true,
                appToken: config.app_token
            });
            
            app.message(async ({ message, say }) => {
                const msg = {
                    type: 'message',
                    id: message.ts,
                    from: message.user,
                    text: message.text,
                    channel: message.channel,
                    timestamp: message.ts
                };
                console.log('MESSAGE:', JSON.stringify(msg));
            });
            
            await app.start();
            console.log('READY');
        } else if (config.action === 'send') {
            await app.client.chat.postMessage({
                channel: config.channel,
                text: config.message
            });
            console.log('SENT:', config.channel);
        }
    } catch (err) {
        console.error('ERROR:', err.message);
    }
});
