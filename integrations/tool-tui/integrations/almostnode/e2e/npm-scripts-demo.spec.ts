import { test, expect } from '@playwright/test';

/**
 * E2E tests for the npm scripts interactive demo page.
 * Tests the interactive terminal UI and container.run() API.
 */
test.describe('npm Scripts Demo', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/examples/npm-scripts-demo.html');
    // Wait for container to initialize
    await page.waitForTimeout(1500);
  });

  test('page loads with terminal and package.json editor', async ({ page }) => {
    // Check that both panels exist
    await expect(page.locator('#pkgEditor')).toBeVisible();
    await expect(page.locator('#terminalInput')).toBeVisible();
    await expect(page.locator('#terminalOutput')).toBeVisible();

    // Check welcome message appears
    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('almostnode npm scripts demo');
  });

  test('typing npm run greet executes the script', async ({ page }) => {
    const input = page.locator('#terminalInput');
    await input.fill('npm run greet');
    await input.press('Enter');

    // Wait for async execution
    await page.waitForTimeout(3000);

    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('$ npm run greet');
    expect(output).toContain('Hello from npm scripts!');
  });

  test('npm run build shows pre/post lifecycle hooks', async ({ page }) => {
    const input = page.locator('#terminalInput');
    await input.fill('npm run build');
    await input.press('Enter');

    await page.waitForTimeout(3000);

    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('Cleaning');
    expect(output).toContain('Building project');
    expect(output).toContain('Build complete');
  });

  test('npm test shorthand works', async ({ page }) => {
    const input = page.locator('#terminalInput');
    await input.fill('npm test');
    await input.press('Enter');

    await page.waitForTimeout(3000);

    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('All tests passed');
  });

  test('basic bash commands work in the terminal', async ({ page }) => {
    const input = page.locator('#terminalInput');
    await input.fill('echo hello world');
    await input.press('Enter');

    await page.waitForTimeout(2000);

    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('hello world');
  });

  test('editing package.json and re-running picks up changes', async ({ page }) => {
    // Modify the package.json to add a new script
    const editor = page.locator('#pkgEditor');
    await editor.fill(JSON.stringify({
      name: 'my-app',
      version: '1.0.0',
      scripts: {
        custom: 'echo custom script works'
      }
    }, null, 2));

    const input = page.locator('#terminalInput');
    await input.fill('npm run custom');
    await input.press('Enter');

    await page.waitForTimeout(3000);

    const output = await page.locator('#terminalOutput').textContent();
    expect(output).toContain('custom script works');
  });
});
