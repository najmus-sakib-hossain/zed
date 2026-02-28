import { test, expect, type Page } from '@playwright/test';

/**
 * Helper to extract text from xterm.js terminal buffer.
 * Uses the exposed window.__term instance for reliable full-content extraction.
 */
async function getTerminalText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const term = (window as any).__term;
    if (!term) return '';
    const buffer = term.buffer.active;
    let text = '';
    for (let i = 0; i <= buffer.cursorY + buffer.baseY; i++) {
      const line = buffer.getLine(i);
      if (line) text += line.translateToString(true) + '\n';
    }
    return text;
  });
}

/**
 * Helper to wait for text to appear in the xterm terminal.
 */
async function waitForTerminalText(page: Page, text: string, timeout = 15000) {
  await page.waitForFunction(
    (searchText: string) => {
      const term = (window as any).__term;
      if (!term) return false;
      const buffer = term.buffer.active;
      let content = '';
      for (let i = 0; i <= buffer.cursorY + buffer.baseY; i++) {
        const line = buffer.getLine(i);
        if (line) content += line.translateToString(true) + '\n';
      }
      return content.includes(searchText);
    },
    text,
    { timeout }
  );
}

/**
 * E2E tests for the Vitest Testing demo page.
 * Tests real vitest execution via npm run test with xterm.js terminal.
 */
test.describe('Vitest Demo', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/examples/vitest-demo.html');
    // Wait for vitest to install (downloads packages)
    await waitForTerminalText(page, 'vitest installed successfully', 30000);
  });

  test('page loads with editor tabs, terminal, and watch checkbox', async ({ page }) => {
    // Check editor tabs exist
    await expect(page.locator('.file-tab[data-file="utils.js"]')).toBeVisible();
    await expect(page.locator('.file-tab[data-file="utils.test.js"]')).toBeVisible();
    await expect(page.locator('.file-tab[data-file="package.json"]')).toBeVisible();

    // Check xterm terminal exists
    await expect(page.locator('#terminal')).toBeVisible();
    await expect(page.locator('.xterm')).toBeVisible();

    // Check watch mode checkbox
    await expect(page.locator('#watchMode')).toBeVisible();

    // Check install success message
    const output = await getTerminalText(page);
    expect(output).toContain('vitest installed successfully');
  });

  test('npm run test executes vitest and all tests pass', async ({ page }) => {
    await page.click('#terminal');
    await page.keyboard.type('npm run test');
    await page.keyboard.press('Enter');

    // Wait for test execution to complete
    await waitForTerminalText(page, 'Duration');

    const output = await getTerminalText(page);
    expect(output).toContain('utils.test.js');
    expect(output).toContain('6 passed');
    expect(output).toContain('Test Files');
  });

  test('switching tabs shows different file contents', async ({ page }) => {
    // utils.js should be active by default
    const editor = page.locator('#editor');
    const initialContent = await editor.inputValue();
    expect(initialContent).toContain('function capitalize');

    // Switch to utils.test.js
    await page.click('.file-tab[data-file="utils.test.js"]');
    await page.waitForTimeout(300);
    const testContent = await editor.inputValue();
    expect(testContent).toContain("require('vitest')");
    expect(testContent).toContain('describe');

    // Switch to package.json
    await page.click('.file-tab[data-file="package.json"]');
    await page.waitForTimeout(300);
    const pkgContent = await editor.inputValue();
    expect(pkgContent).toContain('vitest run');
  });

  test('editing test to fail shows failure output', async ({ page }) => {
    // First run should pass
    await page.click('#terminal');
    await page.keyboard.type('npm run test');
    await page.keyboard.press('Enter');
    await waitForTerminalText(page, 'Duration');

    // Switch to test file and introduce a failing assertion
    await page.click('.file-tab[data-file="utils.test.js"]');
    await page.waitForTimeout(300);

    const editor = page.locator('#editor');
    const content = await editor.inputValue();
    const failingContent = content.replace(
      "expect(sum(1, 2)).toBe(3)",
      "expect(sum(1, 2)).toBe(999)"
    );
    await editor.fill(failingContent);

    // Re-run tests
    await page.click('#terminal');
    await page.keyboard.type('npm run test');
    await page.keyboard.press('Enter');

    // Wait for the second "Duration" to appear
    await page.waitForFunction(
      () => {
        const term = (window as any).__term;
        if (!term) return false;
        const buffer = term.buffer.active;
        let text = '';
        for (let i = 0; i <= buffer.cursorY + buffer.baseY; i++) {
          const line = buffer.getLine(i);
          if (line) text += line.translateToString(true) + '\n';
        }
        const idx1 = text.indexOf('Duration');
        return idx1 >= 0 && text.indexOf('Duration', idx1 + 1) >= 0;
      },
      { timeout: 15000 }
    );

    const output = await getTerminalText(page);
    // Should show failure
    expect(output).toContain('1 failed');
    expect(output).toContain('5 passed');
    expect(output).toContain('expected 3 to be 999');
  });

  test('watch mode re-runs tests when files change', async ({ page }) => {
    // Enable watch mode
    await page.click('#watchMode');

    // Wait for initial test run to complete
    await waitForTerminalText(page, 'Waiting for file changes', 20000);

    // Verify initial run passed
    const initialOutput = await getTerminalText(page);
    expect(initialOutput).toContain('6 passed');

    // Edit a source file and save to trigger restart
    // (vitest restarts to pick up VFS changes since it caches modules internally)
    await page.click('.file-tab[data-file="utils.js"]');
    await page.waitForTimeout(300);
    const editor = page.locator('#editor');
    const content = await editor.inputValue();
    await editor.fill(content + '\n// trigger change');
    await page.click('#saveBtn');

    // Wait for vitest to restart and show results
    // The restart produces a fresh "starting vitest in watch mode" + new results
    await page.waitForFunction(
      () => {
        const term = (window as any).__term;
        if (!term) return false;
        const buffer = term.buffer.active;
        let text = '';
        for (let i = 0; i <= buffer.cursorY + buffer.baseY; i++) {
          const line = buffer.getLine(i);
          if (line) text += line.translateToString(true) + '\n';
        }
        // Look for a second instance of "starting vitest" (restart)
        const idx1 = text.indexOf('starting vitest in watch mode');
        return idx1 >= 0 && text.indexOf('starting vitest in watch mode', idx1 + 1) >= 0;
      },
      { timeout: 20000 }
    );

    // Verify restart completed with passing tests
    await waitForTerminalText(page, 'Waiting for file changes', 10000);
    const rerunOutput = await getTerminalText(page);
    expect(rerunOutput).toContain('6 passed');

    // Disable watch mode
    await page.click('#watchMode');
  });
});
