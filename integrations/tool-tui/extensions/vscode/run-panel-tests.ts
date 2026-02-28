/**
 * Test runner for VS Code extension panel tests
 * 
 * Feature: dx-unified-tooling
 * Task 15: Checkpoint - Ensure VS Code extension tests pass
 */

import { runDrivenPanelTests } from './src/driven/drivenPanel.test';
import { runGeneratorPanelTests } from './src/generator/generatorPanel.test';
import { runDcpPanelTests } from './src/dcp/dcpPanel.test';

async function runAllPanelTests(): Promise<void> {
    console.log('========================================');
    console.log('DX Unified Tooling - VS Code Extension Tests');
    console.log('========================================\n');

    let passed = 0;
    let failed = 0;

    try {
        runDrivenPanelTests();
        passed++;
    } catch (error) {
        console.error('Driven Panel tests failed:', error);
        failed++;
    }

    try {
        runGeneratorPanelTests();
        passed++;
    } catch (error) {
        console.error('Generator Panel tests failed:', error);
        failed++;
    }

    try {
        runDcpPanelTests();
        passed++;
    } catch (error) {
        console.error('DCP Panel tests failed:', error);
        failed++;
    }

    console.log('\n========================================');
    console.log('Test Summary');
    console.log('========================================');
    console.log(`Passed: ${passed}`);
    console.log(`Failed: ${failed}`);
    console.log(`Total: ${passed + failed}`);

    if (failed > 0) {
        process.exit(1);
    }
}

runAllPanelTests();
