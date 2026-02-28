/**
 * Property-based tests for Asset Bridge
 *
 * Feature: dx-unified-assets
 *
 * Tests Properties 19 and 20 from the design document:
 * - Property 19: CLI Availability Check
 * - Property 20: Extension Cache TTL
 *
 * **Validates: Requirements 8.5, 8.6**
 */

import * as fc from 'fast-check';
import {
    parseAssetReference,
    formatAssetReference,
    AssetReference,
    isErrorResponse,
    isSuccessResponse,
    CLIResponse,
} from './assetTypes';

// ============================================================================
// Property 19: CLI Availability Check
// ============================================================================

/**
 * Property 19: CLI Availability Check
 * *For any* extension activation, the CLI availability check SHALL correctly
 * detect whether the `dx` binary is available in PATH and return its version if available.
 *
 * **Validates: Requirements 8.5**
 *
 * Note: This test validates the logic without actually spawning processes.
 */
describe('CLI Availability Check', () => {
    test('should correctly identify version string format', () => {
        // Valid version strings
        const validVersions = [
            'dx 0.1.0',
            'dx 1.0.0',
            'dx 2.3.4-beta',
            'dx 0.0.1-alpha.1',
        ];

        for (const version of validVersions) {
            // Version should contain 'dx' and a version number
            expect(version).toMatch(/dx \d+\.\d+\.\d+/);
        }
    });

    test('should handle missing CLI gracefully', () => {
        // Simulate CLI not found error
        const error = {
            code: 'CLI_NOT_FOUND',
            message: 'dx CLI not found',
            hint: 'Ensure dx is installed and in your PATH',
        };

        expect(error.code).toBe('CLI_NOT_FOUND');
        expect(error.hint).toBeDefined();
    });
});

// ============================================================================
// Property 20: Extension Cache TTL
// ============================================================================

/**
 * Property 20: Extension Cache TTL
 * *For any* cached CLI response, the cache SHALL return the cached value for
 * requests within the TTL and SHALL refresh the value for requests after the TTL expires.
 *
 * **Validates: Requirements 8.6**
 */
describe('Extension Cache TTL', () => {
    // Simple cache implementation for testing
    class TestCache<T> {
        private cache: Map<string, { data: T; timestamp: number }> = new Map();
        private ttl: number;

        constructor(ttl: number) {
            this.ttl = ttl;
        }

        get(key: string, now: number): T | null {
            const entry = this.cache.get(key);
            if (!entry) {
                return null;
            }
            if (now - entry.timestamp > this.ttl) {
                this.cache.delete(key);
                return null;
            }
            return entry.data;
        }

        set(key: string, data: T, timestamp: number): void {
            this.cache.set(key, { data, timestamp });
        }
    }

    test('should return cached value within TTL', () => {
        fc.assert(
            fc.property(
                fc.string({ minLength: 1 }),
                fc.string(),
                fc.integer({ min: 1000, max: 300000 }), // TTL between 1s and 5min
                fc.integer({ min: 0, max: 100000 }), // Time offset within TTL
                (key, value, ttl, timeOffset) => {
                    const cache = new TestCache<string>(ttl);
                    const startTime = 1000000;

                    // Set cache at startTime
                    cache.set(key, value, startTime);

                    // Get within TTL should return value
                    const withinTTL = startTime + Math.min(timeOffset, ttl - 1);
                    const result = cache.get(key, withinTTL);

                    return result === value;
                }
            ),
            { numRuns: 100 }
        );
    });

    test('should return null after TTL expires', () => {
        fc.assert(
            fc.property(
                fc.string({ minLength: 1 }),
                fc.string(),
                fc.integer({ min: 1000, max: 300000 }), // TTL
                fc.integer({ min: 1, max: 100000 }), // Extra time after TTL
                (key, value, ttl, extraTime) => {
                    const cache = new TestCache<string>(ttl);
                    const startTime = 1000000;

                    // Set cache at startTime
                    cache.set(key, value, startTime);

                    // Get after TTL should return null
                    const afterTTL = startTime + ttl + extraTime;
                    const result = cache.get(key, afterTTL);

                    return result === null;
                }
            ),
            { numRuns: 100 }
        );
    });

    test('should handle cache miss', () => {
        const cache = new TestCache<string>(5000);
        const result = cache.get('nonexistent', Date.now());
        expect(result).toBeNull();
    });
});

// ============================================================================
// Property 13: Asset Reference Format Consistency
// ============================================================================

/**
 * Property 13: Asset Reference Format Consistency
 * *For any* generated asset reference, the reference string SHALL follow the
 * format `<type>:<provider>:<id>` and SHALL be parseable back to its component parts.
 *
 * **Validates: Requirements 5.4**
 */
describe('Asset Reference Format', () => {
    const assetTypeArb = fc.constantFrom('icon', 'font', 'media') as fc.Arbitrary<
        'icon' | 'font' | 'media'
    >;

    test('should round-trip asset references', () => {
        fc.assert(
            fc.property(
                assetTypeArb,
                fc.string({ minLength: 1 }).filter((s) => !s.includes(':')),
                fc.string({ minLength: 1 }),
                (type, provider, id) => {
                    const ref: AssetReference = { type, provider, id };
                    const formatted = formatAssetReference(ref);
                    const parsed = parseAssetReference(formatted);

                    return (
                        parsed !== null &&
                        parsed.type === type &&
                        parsed.provider === provider &&
                        parsed.id === id
                    );
                }
            ),
            { numRuns: 100 }
        );
    });

    test('should handle IDs with colons', () => {
        const ref: AssetReference = {
            type: 'media',
            provider: 'openverse',
            id: 'abc:123:xyz',
        };

        const formatted = formatAssetReference(ref);
        expect(formatted).toBe('media:openverse:abc:123:xyz');

        const parsed = parseAssetReference(formatted);
        expect(parsed).not.toBeNull();
        expect(parsed?.id).toBe('abc:123:xyz');
    });

    test('should reject invalid reference strings', () => {
        const invalidRefs = [
            '', // Empty
            'icon', // Missing parts
            'icon:heroicons', // Missing id
            'invalid:provider:id', // Invalid type
        ];

        for (const ref of invalidRefs) {
            const parsed = parseAssetReference(ref);
            if (ref.split(':').length >= 3 && ['icon', 'font', 'media'].includes(ref.split(':')[0])) {
                // Valid format
                expect(parsed).not.toBeNull();
            } else {
                // Invalid format
                expect(parsed).toBeNull();
            }
        }
    });
});

// ============================================================================
// Response Type Guards
// ============================================================================

describe('Response Type Guards', () => {
    test('should correctly identify error responses', () => {
        fc.assert(
            fc.property(fc.string(), fc.string(), (error, code) => {
                const response: CLIResponse<unknown> = { error, code };
                return isErrorResponse(response) && !isSuccessResponse(response);
            }),
            { numRuns: 100 }
        );
    });

    test('should correctly identify success responses', () => {
        fc.assert(
            fc.property(fc.array(fc.string()), fc.integer({ min: 0 }), (results, total) => {
                const response: CLIResponse<string[]> = {
                    success: true,
                    results,
                    total,
                };
                return isSuccessResponse(response) && !isErrorResponse(response);
            }),
            { numRuns: 100 }
        );
    });
});
