/**
 * Tests for LLM to Human format conversion
 * 
 * LLM Format patterns:
 * 1. Root scalars: key=value key2=value2
 * 2. Inline objects: section:count[key=value key2[count]=item1 item2]
 * 3. Key-value pairs: section:count@=[key value key value]
 * 4. Simple arrays: section:count=item1 item2 item3
 * 5. Tabular data: section:count(schema)@suffixes[row1;row2]
 * 
 * Suffix patterns in tabular:
 * - @prefix - prepend to column value (position-based, left to right)
 * - @@suffix - append to column value
 * - Suffixes are applied in order to columns that have compressed values
 */

import { llmToHumanFallback } from './conversion';

describe('LLM to Human Conversion', () => {
    
    describe('Root Scalars', () => {
        test('single scalar', () => {
            const input = 'name=dx-project';
            const output = llmToHumanFallback(input);
            expect(output).toContain('name');
            expect(output).toContain('dx-project');
        });

        test('multiple scalars on same line', () => {
            const input = 'name=dx-project version=1.2.3 author=essensefromexistence';
            const output = llmToHumanFallback(input);
            expect(output).toContain('name');
            expect(output).toContain('dx-project');
            expect(output).toContain('version');
            expect(output).toContain('1.2.3');
            expect(output).toContain('author');
            expect(output).toContain('essensefromexistence');
        });

        test('underscores converted to spaces', () => {
            const input = 'title=Hello_World';
            const output = llmToHumanFallback(input);
            expect(output).toContain('Hello World');
        });
    });

    describe('Inline Objects', () => {
        test('simple inline object', () => {
            const input = 'context:3[task=Hiking_trip location=Boulder season=spring]';
            const output = llmToHumanFallback(input);
            expect(output).toContain('[context]');
            expect(output).toContain('task');
            expect(output).toContain('Hiking trip');
            expect(output).toContain('location');
            expect(output).toContain('Boulder');
            expect(output).toContain('season');
            expect(output).toContain('spring');
        });

        test('inline object with nested array', () => {
            const input = 'workspace:2[paths[4]=@/www @/backend @/mobile @/shared default=@/www]';
            const output = llmToHumanFallback(input);
            expect(output).toContain('[workspace]');
            expect(output).toContain('paths');
            expect(output).toContain('@/www');
            expect(output).toContain('@/backend');
            expect(output).toContain('default');
        });
    });

    describe('Key-Value Pairs (@=)', () => {
        test('simple key-value pairs', () => {
            const input = 'server:6@=[host localhost port 8080 debug true maxConn 100 timeout 30000 workers 4]';
            const output = llmToHumanFallback(input);
            expect(output).toContain('[server]');
            expect(output).toContain('host');
            expect(output).toContain('localhost');
            expect(output).toContain('port');
            expect(output).toContain('8080');
            expect(output).toContain('debug');
            expect(output).toContain('true');
        });
    });

    describe('Simple Arrays', () => {
        test('simple array with =', () => {
            const input = 'friends:8=ana luis sam maria carlos emma david sophia';
            const output = llmToHumanFallback(input);
            expect(output).toContain('[friends]');
            expect(output).toContain('- ana');
            expect(output).toContain('- luis');
            expect(output).toContain('- sophia');
        });
    });

    describe('Tabular Data with Suffix Expansion', () => {
        
        test('example3: orders with @ORD- @2025-01- @@ex.com suffixes', () => {
            const input = 'orders:8(id customer email items total status date)@ORD- @2025-01- @@ex.com[001 Alice_Chen alice@ WIDGET:2:30|GADGET:1:50 110 shipped 10;002 Bob_Smith bob@ THING:3:15 45 delivered 11]';
            const output = llmToHumanFallback(input);
            
            // Check section headers
            expect(output).toContain('[orders:1]');
            expect(output).toContain('[orders:2]');
            
            // Check first order - suffixes should be expanded
            // @ORD- applies to id column: 001 -> ORD-001
            expect(output).toContain('ORD-001');
            // @2025-01- applies to date column: 10 -> 2025-01-10
            expect(output).toContain('2025-01-10');
            // @@ex.com applies to email column: alice@ -> alice@ex.com
            expect(output).toContain('alice@ex.com');
            // customer should have underscore converted: Alice_Chen -> Alice Chen
            expect(output).toContain('Alice Chen');
            
            // Check second order
            expect(output).toContain('ORD-002');
            expect(output).toContain('2025-01-11');
            expect(output).toContain('bob@ex.com');
            expect(output).toContain('Bob Smith');
        });

        test('example2: logs with @/api/ @2025-01-15T suffixes', () => {
            const input = 'logs:12(timestamp level endpoint status time error)@/api/ @2025-01-15T[10:23:45Z info users 200 45 null:10:24:12Z error orders 500 120 Database_timeout]';
            const output = llmToHumanFallback(input);
            
            // Check section headers
            expect(output).toContain('[logs:1]');
            
            // @2025-01-15T applies to timestamp: 10:23:45Z -> 2025-01-15T10:23:45Z
            expect(output).toContain('2025-01-15T10:23:45Z');
            // @/api/ applies to endpoint: users -> /api/users
            expect(output).toContain('/api/users');
            // error should have underscore converted: Database_timeout -> Database timeout
            expect(output).toContain('Database timeout');
        });

        test('example1: hikes with @_Trail @4. suffixes', () => {
            // Note: The original example1_mixed.sr seems corrupted, using expected format
            const input = 'hikes:10(id name dist elev companion sunny rating)@_Trail @4.[1 Blue_Lake 7.5 320 ana true 5, 2 Ridge_View 9.2 540 luis false 2]';
            const output = llmToHumanFallback(input);
            
            // Check section headers
            expect(output).toContain('[hikes:1]');
            expect(output).toContain('[hikes:2]');
            
            // @_Trail applies to name: Blue_Lake -> Blue Lake Trail (underscore to space, then append Trail)
            expect(output).toContain('Blue Lake Trail');
            // @4. applies to rating: 5 -> 4.5
            expect(output).toContain('4.5');
            
            // Second hike
            expect(output).toContain('Ridge View Trail');
            expect(output).toContain('4.2');
        });
    });

    describe('Full File Examples', () => {
        
        test('example4_config.sr - key-value pairs format', () => {
            const input = `server:6@=[host localhost port 8080 debug true maxConn 100 timeout 30000 workers 4]
database:5@=[url postgres://localhost:5432/db pool 10 timeout 30 ssl true maxRetries 3]
cache:4@=[enabled true ttl 3600 maxSize 1000 strategy lru]`;
            
            const output = llmToHumanFallback(input);
            
            expect(output).toContain('[server]');
            expect(output).toContain('host');
            expect(output).toContain('localhost');
            expect(output).toContain('[database]');
            expect(output).toContain('postgres://localhost:5432/db');
            expect(output).toContain('[cache]');
            expect(output).toContain('strategy');
            expect(output).toContain('lru');
        });

        test('example5_leaf.sr - mixed format', () => {
            const input = `name=dx-project version=1.2.3 author=essensefromexistence
workspace:2[paths[4]=@/www @/backend @/mobile @/shared default=@/www]
editors:2[items[5]=neovim zed vscode cursor windsurf default=neovim]`;
            
            const output = llmToHumanFallback(input);
            
            // Root scalars
            expect(output).toContain('name');
            expect(output).toContain('dx-project');
            expect(output).toContain('version');
            expect(output).toContain('1.2.3');
            
            // Sections
            expect(output).toContain('[workspace]');
            expect(output).toContain('[editors]');
        });
    });
});

// Export for running
export {};
