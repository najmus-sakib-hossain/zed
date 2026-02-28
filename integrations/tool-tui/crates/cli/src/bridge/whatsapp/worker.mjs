#!/usr/bin/env bun
// WhatsApp bridge using whatsapp-web.js

import { Client, LocalAuth } from 'whatsapp-web.js';
import qrcode from 'qrcode-terminal';

const client = new Client({
    authStrategy: new LocalAuth(),
    puppeteer: {
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    }
});

client.on('qr', (qr) => {
    console.log('QR_CODE:', qr);
    qrcode.generate(qr, { small: true });
});

client.on('ready', () => {
    console.log('READY');
});

client.on('message', async (msg) => {
    const data = {
        type: 'message',
        id: msg.id._serialized,
        from: msg.from,
        body: msg.body,
        timestamp: msg.timestamp,
        hasMedia: msg.hasMedia
    };
    console.log('MESSAGE:', JSON.stringify(data));
});

client.on('authenticated', () => {
    console.log('AUTHENTICATED');
});

client.on('auth_failure', (msg) => {
    console.error('AUTH_FAILURE:', msg);
});

client.on('disconnected', (reason) => {
    console.log('DISCONNECTED:', reason);
});

// Handle stdin for commands
process.stdin.on('data', async (data) => {
    try {
        const cmd = JSON.parse(data.toString());
        
        if (cmd.action === 'send') {
            await client.sendMessage(cmd.recipient, cmd.message);
            console.log('SENT:', cmd.recipient);
        }
    } catch (err) {
        console.error('ERROR:', err.message);
    }
});

client.initialize();
