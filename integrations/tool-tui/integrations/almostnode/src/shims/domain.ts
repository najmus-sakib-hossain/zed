/**
 * Node.js domain module shim
 * The domain module is deprecated but some packages still use it
 */

import { EventEmitter } from './events';

export class Domain extends EventEmitter {
  members: unknown[] = [];

  add(emitter: EventEmitter): void {
    this.members.push(emitter);
  }

  remove(emitter: EventEmitter): void {
    const index = this.members.indexOf(emitter);
    if (index !== -1) {
      this.members.splice(index, 1);
    }
  }

  bind<T extends (...args: unknown[]) => unknown>(callback: T): T {
    return callback;
  }

  intercept<T extends (...args: unknown[]) => unknown>(callback: T): T {
    return callback;
  }

  run<T>(fn: () => T): T {
    return fn();
  }

  dispose(): void {
    this.members = [];
  }

  enter(): void {
    // Stub
  }

  exit(): void {
    // Stub
  }
}

export function create(): Domain {
  return new Domain();
}

// Active domain (deprecated but some packages check for it)
export let active: Domain | null = null;

export default {
  Domain,
  create,
  active,
};
