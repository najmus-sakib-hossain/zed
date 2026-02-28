import { test, expect } from '@playwright/test';

/**
 * E2E tests for the npm command in child_process.
 * Tests npm run, npm start, npm test, lifecycle scripts, and error handling.
 * Uses bash-demo.html as the test harness.
 */
test.describe('npm command E2E', () => {
  /**
   * Helper: fill the editor with a script, run it, and collect console output.
   * Returns collected console messages after waiting for execution.
   */
  async function runScript(page: ReturnType<typeof test['_options']>['page'] & any, script: string): Promise<string[]> {
    const messages: string[] = [];
    page.on('console', (msg: any) => {
      messages.push(msg.text());
    });

    // Fill editor and run
    const editor = page.locator('#editor');
    await editor.fill(script);
    await page.click('#runBtn');

    // Wait for async bash execution to complete
    await page.waitForTimeout(3000);

    return messages;
  }

  test.beforeEach(async ({ page }) => {
    await page.goto('/examples/bash-demo.html');
    // Wait for container to initialize
    await page.waitForTimeout(1000);
  });

  test('npm run executes a script from package.json', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  version: '1.0.0',
  scripts: { hello: 'echo hello from npm script' }
}));

exec('npm run hello', (err, stdout, stderr) => {
  console.log('RESULT:' + stdout.trim());
  if (err) console.log('ERROR:' + err.message);
});
    `);

    expect(messages.some(m => m.includes('hello from npm script'))).toBe(true);
  });

  test('npm start shorthand works', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { start: 'echo app started successfully' }
}));

exec('npm start', (err, stdout, stderr) => {
  console.log('RESULT:' + stdout.trim());
});
    `);

    expect(messages.some(m => m.includes('app started successfully'))).toBe(true);
  });

  test('npm test shorthand works', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { test: 'echo all tests passed' }
}));

exec('npm test', (err, stdout, stderr) => {
  console.log('RESULT:' + stdout.trim());
});
    `);

    expect(messages.some(m => m.includes('all tests passed'))).toBe(true);
  });

  test('npm run with no args lists available scripts', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { build: 'echo build', dev: 'echo dev', lint: 'echo lint' }
}));

exec('npm run', (err, stdout, stderr) => {
  console.log('SCRIPTS:' + stdout);
});
    `);

    expect(messages.some(m => m.includes('build') && m.includes('dev') && m.includes('lint'))).toBe(true);
  });

  test('missing script returns error', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { build: 'echo build' }
}));

exec('npm run nonexistent', (err, stdout, stderr) => {
  console.log('STDERR:' + stderr);
  if (err) console.log('FAILED:true');
});
    `);

    expect(messages.some(m => m.includes('Missing script') && m.includes('nonexistent'))).toBe(true);
    expect(messages.some(m => m.includes('FAILED:true'))).toBe(true);
  });

  test('pre/post lifecycle scripts execute in order', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: {
    prebuild: 'echo PHASE:pre',
    build: 'echo PHASE:main',
    postbuild: 'echo PHASE:post'
  }
}));

exec('npm run build', (err, stdout, stderr) => {
  console.log('OUTPUT:' + stdout);
});
    `);

    const outputMsg = messages.find(m => m.startsWith('OUTPUT:'));
    expect(outputMsg).toBeDefined();
    const output = outputMsg!;
    const preIdx = output.indexOf('PHASE:pre');
    const mainIdx = output.indexOf('PHASE:main');
    const postIdx = output.indexOf('PHASE:post');
    expect(preIdx).toBeGreaterThanOrEqual(0);
    expect(mainIdx).toBeGreaterThan(preIdx);
    expect(postIdx).toBeGreaterThan(mainIdx);
  });

  test('scripts with shell features work', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { combo: 'echo first && echo second' }
}));

exec('npm run combo', (err, stdout, stderr) => {
  console.log('OUTPUT:' + stdout);
});
    `);

    expect(messages.some(m => m.includes('first') && m.includes('second'))).toBe(true);
  });

  test('script that calls node works', async ({ page }) => {
    const messages = await runScript(page, `
const fs = require('fs');
const { exec } = require('child_process');

fs.writeFileSync('/hello.js', 'console.log("node executed successfully");');
fs.writeFileSync('/package.json', JSON.stringify({
  name: 'test-app',
  scripts: { start: 'node /hello.js' }
}));

exec('npm start', (err, stdout, stderr) => {
  console.log('OUTPUT:' + stdout);
});
    `);

    expect(messages.some(m => m.includes('node executed successfully'))).toBe(true);
  });
});
