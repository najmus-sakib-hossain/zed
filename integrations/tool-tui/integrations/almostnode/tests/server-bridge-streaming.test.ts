import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { ServerBridge, getServerBridge, resetServerBridge } from '../src/server-bridge';
import { VirtualFS } from '../src/virtual-fs';
import { NextDevServer } from '../src/frameworks/next-dev-server';
import { Buffer } from '../src/shims/stream';

describe('ServerBridge streaming support', () => {
  let bridge: ServerBridge;
  let vfs: VirtualFS;
  let server: NextDevServer;

  beforeEach(() => {
    resetServerBridge();
    bridge = getServerBridge();
    vfs = new VirtualFS();

    // Set up a minimal Next.js project with streaming API
    vfs.mkdirSync('/pages', { recursive: true });
    vfs.mkdirSync('/pages/api', { recursive: true });

    vfs.writeFileSync('/pages/index.jsx', '<div>Test</div>');

    // Create a streaming API route
    vfs.writeFileSync(
      '/pages/api/stream.js',
      `export default function handler(req, res) {
  res.setHeader('Content-Type', 'text/event-stream');
  res.write('data: chunk1\\n\\n');
  res.write('data: chunk2\\n\\n');
  res.write('data: chunk3\\n\\n');
  res.end();
}
`
    );

    // Create a regular JSON API route
    vfs.writeFileSync(
      '/pages/api/json.js',
      `export default function handler(req, res) {
  res.status(200).json({ message: 'hello' });
}
`
    );

    server = new NextDevServer(vfs, { port: 3001 });
    bridge.registerServer(server as any, 3001);
  });

  afterEach(() => {
    server.stop();
    resetServerBridge();
  });

  describe('handleRequest (non-streaming)', () => {
    it('should handle regular requests', async () => {
      const response = await bridge.handleRequest(
        3001,
        'GET',
        '/api/json',
        {},
        undefined
      );

      expect(response.statusCode).toBe(200);
      expect(response.headers['Content-Type']).toBe('application/json; charset=utf-8');

      const body = JSON.parse(response.body.toString());
      expect(body).toEqual({ message: 'hello' });
    });

    it('should return 503 for unregistered ports', async () => {
      const response = await bridge.handleRequest(
        9999,
        'GET',
        '/api/test',
        {},
        undefined
      );

      expect(response.statusCode).toBe(503);
      expect(response.body.toString()).toContain('No server listening');
    });
  });

  describe('server with handleStreamingRequest support', () => {
    it('should have handleStreamingRequest method available', () => {
      expect(typeof server.handleStreamingRequest).toBe('function');
    });

    it('should stream chunks through handleStreamingRequest', async () => {
      const chunks: string[] = [];
      let startCalled = false;
      let endCalled = false;
      let statusCode = 0;
      let headers: Record<string, string> = {};

      await server.handleStreamingRequest(
        'GET',
        '/api/stream',
        {},
        undefined,
        (code, message, hdrs) => {
          startCalled = true;
          statusCode = code;
          headers = hdrs;
        },
        (chunk) => {
          chunks.push(typeof chunk === 'string' ? chunk : new TextDecoder().decode(chunk));
        },
        () => {
          endCalled = true;
        }
      );

      expect(startCalled).toBe(true);
      expect(endCalled).toBe(true);
      expect(statusCode).toBe(200);
      expect(headers['Content-Type']).toBe('text/event-stream');
      expect(chunks.length).toBe(3);
      expect(chunks[0]).toBe('data: chunk1\n\n');
      expect(chunks[1]).toBe('data: chunk2\n\n');
      expect(chunks[2]).toBe('data: chunk3\n\n');
    });
  });
});

describe('ServerBridge message protocol for streaming', () => {
  // These tests verify the protocol structure used for streaming
  // The actual message passing is tested via integration tests

  it('should define stream-start message format', () => {
    const streamStartMessage = {
      type: 'stream-start',
      id: 1,
      data: {
        statusCode: 200,
        statusMessage: 'OK',
        headers: { 'Content-Type': 'text/plain' },
      },
    };

    expect(streamStartMessage.type).toBe('stream-start');
    expect(streamStartMessage.data.statusCode).toBe(200);
    expect(streamStartMessage.data.headers).toBeDefined();
  });

  it('should define stream-chunk message format', () => {
    const chunk = 'Hello, World!';
    const bytes = new TextEncoder().encode(chunk);
    let binary = '';
    for (let i = 0; i < bytes.length; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    const chunkBase64 = btoa(binary);

    const streamChunkMessage = {
      type: 'stream-chunk',
      id: 1,
      data: {
        chunkBase64,
      },
    };

    expect(streamChunkMessage.type).toBe('stream-chunk');
    expect(streamChunkMessage.data.chunkBase64).toBe(chunkBase64);

    // Verify we can decode it back
    const decoded = atob(streamChunkMessage.data.chunkBase64);
    expect(decoded).toBe(chunk);
  });

  it('should define stream-end message format', () => {
    const streamEndMessage = {
      type: 'stream-end',
      id: 1,
    };

    expect(streamEndMessage.type).toBe('stream-end');
    expect(streamEndMessage.id).toBe(1);
  });
});

describe('Streaming response accumulation', () => {
  let vfs: VirtualFS;
  let server: NextDevServer;

  beforeEach(() => {
    vfs = new VirtualFS();
    vfs.mkdirSync('/pages/api', { recursive: true });

    // Create an API that streams multiple chunks
    vfs.writeFileSync(
      '/pages/api/multi-chunk.js',
      `export default function handler(req, res) {
  res.setHeader('Content-Type', 'text/plain');
  for (let i = 1; i <= 5; i++) {
    res.write(\`chunk\${i}\`);
  }
  res.end();
}
`
    );

    server = new NextDevServer(vfs, { port: 3001 });
  });

  afterEach(() => {
    server.stop();
  });

  it('should accumulate all chunks in correct order', async () => {
    const chunks: string[] = [];

    await server.handleStreamingRequest(
      'GET',
      '/api/multi-chunk',
      {},
      undefined,
      () => {},
      (chunk) => {
        chunks.push(typeof chunk === 'string' ? chunk : chunk.toString());
      },
      () => {}
    );

    expect(chunks).toEqual(['chunk1', 'chunk2', 'chunk3', 'chunk4', 'chunk5']);
  });

  it('should produce same content as non-streaming request', async () => {
    const streamedChunks: string[] = [];

    await server.handleStreamingRequest(
      'GET',
      '/api/multi-chunk',
      {},
      undefined,
      () => {},
      (chunk) => {
        streamedChunks.push(typeof chunk === 'string' ? chunk : chunk.toString());
      },
      () => {}
    );

    const nonStreamedResponse = await server.handleRequest('GET', '/api/multi-chunk', {});

    expect(streamedChunks.join('')).toBe(nonStreamedResponse.body.toString());
  });
});

describe('Streaming error handling', () => {
  let vfs: VirtualFS;
  let server: NextDevServer;

  beforeEach(() => {
    vfs = new VirtualFS();
    vfs.mkdirSync('/pages/api', { recursive: true });

    // Create an API that throws an error
    vfs.writeFileSync(
      '/pages/api/error.js',
      `export default function handler(req, res) {
  throw new Error('Something went wrong');
}
`
    );

    server = new NextDevServer(vfs, { port: 3001 });
  });

  afterEach(() => {
    server.stop();
  });

  it('should call onStart and onEnd even on errors', async () => {
    let startCalled = false;
    let endCalled = false;
    let statusCode = 0;

    await server.handleStreamingRequest(
      'GET',
      '/api/error',
      {},
      undefined,
      (code) => {
        startCalled = true;
        statusCode = code;
      },
      () => {},
      () => {
        endCalled = true;
      }
    );

    expect(startCalled).toBe(true);
    expect(endCalled).toBe(true);
    expect(statusCode).toBe(500);
  });

  it('should include error message in chunk on handler error', async () => {
    const chunks: string[] = [];

    await server.handleStreamingRequest(
      'GET',
      '/api/error',
      {},
      undefined,
      () => {},
      (chunk) => {
        chunks.push(typeof chunk === 'string' ? chunk : chunk.toString());
      },
      () => {}
    );

    expect(chunks.length).toBeGreaterThan(0);
    const errorResponse = JSON.parse(chunks[0]);
    expect(errorResponse.error).toContain('Something went wrong');
  });
});
