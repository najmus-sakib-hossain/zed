/**
 * Node.js diagnostics_channel module shim
 * Provides basic diagnostics channel functionality for packages like undici
 */

import { EventEmitter } from './events';

/**
 * A Channel is used to publish messages to subscribers
 */
export class Channel {
  private name: string;
  private _subscribers: Set<(message: unknown, name: string) => void> = new Set();

  constructor(name: string) {
    this.name = name;
  }

  get hasSubscribers(): boolean {
    return this._subscribers.size > 0;
  }

  publish(message: unknown): void {
    for (const subscriber of this._subscribers) {
      try {
        subscriber(message, this.name);
      } catch (err) {
        console.error('Error in diagnostics channel subscriber:', err);
      }
    }
  }

  subscribe(onMessage: (message: unknown, name: string) => void): void {
    this._subscribers.add(onMessage);
  }

  unsubscribe(onMessage: (message: unknown, name: string) => void): boolean {
    return this._subscribers.delete(onMessage);
  }

  bindStore(store: unknown, transform?: (message: unknown) => unknown): void {
    // Stub - AsyncLocalStorage integration not implemented
  }

  unbindStore(store: unknown): boolean {
    return false;
  }
}

// Channel registry
const channels = new Map<string, Channel>();

/**
 * Get or create a channel by name
 */
export function channel(name: string): Channel {
  let ch = channels.get(name);
  if (!ch) {
    ch = new Channel(name);
    channels.set(name, ch);
  }
  return ch;
}

/**
 * Check if a channel has subscribers
 */
export function hasSubscribers(name: string): boolean {
  const ch = channels.get(name);
  return ch ? ch.hasSubscribers : false;
}

/**
 * Subscribe to a channel
 */
export function subscribe(name: string, onMessage: (message: unknown, name: string) => void): void {
  channel(name).subscribe(onMessage);
}

/**
 * Unsubscribe from a channel
 */
export function unsubscribe(name: string, onMessage: (message: unknown, name: string) => void): boolean {
  const ch = channels.get(name);
  return ch ? ch.unsubscribe(onMessage) : false;
}

/**
 * TracingChannel for distributed tracing
 */
export class TracingChannel {
  private channels: {
    start: Channel;
    end: Channel;
    asyncStart: Channel;
    asyncEnd: Channel;
    error: Channel;
  };

  constructor(nameOrChannels: string | { start: Channel; end: Channel; asyncStart: Channel; asyncEnd: Channel; error: Channel }) {
    if (typeof nameOrChannels === 'string') {
      this.channels = {
        start: channel(`tracing:${nameOrChannels}:start`),
        end: channel(`tracing:${nameOrChannels}:end`),
        asyncStart: channel(`tracing:${nameOrChannels}:asyncStart`),
        asyncEnd: channel(`tracing:${nameOrChannels}:asyncEnd`),
        error: channel(`tracing:${nameOrChannels}:error`),
      };
    } else {
      this.channels = nameOrChannels;
    }
  }

  get hasSubscribers(): boolean {
    return Object.values(this.channels).some(ch => ch.hasSubscribers);
  }

  subscribe(handlers: {
    start?: (message: unknown) => void;
    end?: (message: unknown) => void;
    asyncStart?: (message: unknown) => void;
    asyncEnd?: (message: unknown) => void;
    error?: (message: unknown) => void;
  }): void {
    if (handlers.start) this.channels.start.subscribe(handlers.start);
    if (handlers.end) this.channels.end.subscribe(handlers.end);
    if (handlers.asyncStart) this.channels.asyncStart.subscribe(handlers.asyncStart);
    if (handlers.asyncEnd) this.channels.asyncEnd.subscribe(handlers.asyncEnd);
    if (handlers.error) this.channels.error.subscribe(handlers.error);
  }

  unsubscribe(handlers: {
    start?: (message: unknown) => void;
    end?: (message: unknown) => void;
    asyncStart?: (message: unknown) => void;
    asyncEnd?: (message: unknown) => void;
    error?: (message: unknown) => void;
  }): void {
    if (handlers.start) this.channels.start.unsubscribe(handlers.start);
    if (handlers.end) this.channels.end.unsubscribe(handlers.end);
    if (handlers.asyncStart) this.channels.asyncStart.unsubscribe(handlers.asyncStart);
    if (handlers.asyncEnd) this.channels.asyncEnd.unsubscribe(handlers.asyncEnd);
    if (handlers.error) this.channels.error.unsubscribe(handlers.error);
  }

  traceSync<T>(fn: () => T, context?: unknown, thisArg?: unknown): T {
    this.channels.start.publish(context);
    try {
      const result = fn.call(thisArg);
      this.channels.end.publish(context);
      return result;
    } catch (error) {
      this.channels.error.publish({ error, ...context as object });
      throw error;
    }
  }

  async tracePromise<T>(fn: () => Promise<T>, context?: unknown, thisArg?: unknown): Promise<T> {
    this.channels.start.publish(context);
    try {
      const result = await fn.call(thisArg);
      this.channels.asyncEnd.publish(context);
      return result;
    } catch (error) {
      this.channels.error.publish({ error, ...context as object });
      throw error;
    }
  }

  traceCallback<T extends (...args: unknown[]) => unknown>(
    fn: T,
    position?: number,
    context?: unknown,
    thisArg?: unknown
  ): T {
    // Simplified callback tracing
    return fn;
  }
}

/**
 * Create a TracingChannel
 */
export function tracingChannel(name: string): TracingChannel {
  return new TracingChannel(name);
}

export default {
  channel,
  hasSubscribers,
  subscribe,
  unsubscribe,
  tracingChannel,
  Channel,
  TracingChannel,
};
