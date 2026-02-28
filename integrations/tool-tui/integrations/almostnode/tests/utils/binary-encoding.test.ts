import { describe, it, expect } from 'vitest';
import { uint8ToBase64, base64ToUint8, uint8ToHex, uint8ToBinaryString } from '../../src/utils/binary-encoding';

describe('binary-encoding', () => {
  describe('uint8ToBase64 / base64ToUint8 round-trip', () => {
    it('should handle empty array', () => {
      const bytes = new Uint8Array(0);
      const base64 = uint8ToBase64(bytes);
      expect(base64).toBe('');
      const result = base64ToUint8(base64);
      expect(result.length).toBe(0);
    });

    it('should handle simple ASCII string', () => {
      const bytes = new TextEncoder().encode('hello world');
      const base64 = uint8ToBase64(bytes);
      expect(base64).toBe(btoa('hello world'));
      const result = base64ToUint8(base64);
      expect(new TextDecoder().decode(result)).toBe('hello world');
    });

    it('should handle binary data with all byte values', () => {
      const bytes = new Uint8Array(256);
      for (let i = 0; i < 256; i++) bytes[i] = i;
      const base64 = uint8ToBase64(bytes);
      const result = base64ToUint8(base64);
      expect(result).toEqual(bytes);
    });

    it('should handle large arrays (100KB+)', () => {
      const bytes = new Uint8Array(150_000);
      for (let i = 0; i < bytes.length; i++) bytes[i] = i % 256;
      const base64 = uint8ToBase64(bytes);
      const result = base64ToUint8(base64);
      expect(result).toEqual(bytes);
    });

    it('should handle single byte', () => {
      const bytes = new Uint8Array([42]);
      const base64 = uint8ToBase64(bytes);
      const result = base64ToUint8(base64);
      expect(result).toEqual(bytes);
    });
  });

  describe('uint8ToHex', () => {
    it('should handle empty array', () => {
      expect(uint8ToHex(new Uint8Array(0))).toBe('');
    });

    it('should convert bytes to hex', () => {
      const bytes = new Uint8Array([0x68, 0x65, 0x6c, 0x6c, 0x6f]);
      expect(uint8ToHex(bytes)).toBe('68656c6c6f');
    });

    it('should pad single-digit hex values', () => {
      const bytes = new Uint8Array([0, 1, 15, 16, 255]);
      expect(uint8ToHex(bytes)).toBe('00010f10ff');
    });

    it('should handle all byte values', () => {
      const bytes = new Uint8Array(256);
      for (let i = 0; i < 256; i++) bytes[i] = i;
      const hex = uint8ToHex(bytes);
      expect(hex.length).toBe(512);
      expect(hex.slice(0, 6)).toBe('000102');
      expect(hex.slice(-6)).toBe('fdfeff');
    });
  });

  describe('uint8ToBinaryString', () => {
    it('should handle empty array', () => {
      expect(uint8ToBinaryString(new Uint8Array(0))).toBe('');
    });

    it('should convert bytes to binary string', () => {
      const bytes = new Uint8Array([72, 101, 108, 108, 111]);
      expect(uint8ToBinaryString(bytes)).toBe('Hello');
    });

    it('should handle high byte values (latin1)', () => {
      const bytes = new Uint8Array([0xc0, 0xff, 0x00]);
      const str = uint8ToBinaryString(bytes);
      expect(str.charCodeAt(0)).toBe(0xc0);
      expect(str.charCodeAt(1)).toBe(0xff);
      expect(str.charCodeAt(2)).toBe(0x00);
    });

    it('should handle large arrays without stack overflow', () => {
      const bytes = new Uint8Array(100_000);
      for (let i = 0; i < bytes.length; i++) bytes[i] = i % 256;
      const str = uint8ToBinaryString(bytes);
      expect(str.length).toBe(100_000);
    });
  });
});
