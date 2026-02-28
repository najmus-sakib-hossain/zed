/**
 * child_process integration tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { VirtualFS } from '../src/virtual-fs';
import { Runtime } from '../src/runtime';

describe('child_process Integration', () => {
  let vfs: VirtualFS;
  let runtime: Runtime;
  let consoleOutput: string[] = [];

  beforeEach(() => {
    vfs = new VirtualFS();
    consoleOutput = [];
    runtime = new Runtime(vfs, {
      onConsole: (method, args) => {
        consoleOutput.push(args.join(' '));
      },
    });
  });

  describe('exec', () => {
    it('should execute echo command', async () => {
      // Create a test file
      vfs.writeFileSync('/test.txt', 'hello world');

      const code = `
const { exec } = require('child_process');

exec('echo "Hello from bash"', (error, stdout, stderr) => {
  if (error) {
    console.log('error:', error.message);
    return;
  }
  console.log('stdout:', stdout.trim());
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(consoleOutput.some(o => o.includes('Hello from bash'))).toBe(true);
    });

    it('should execute ls command', async () => {
      // Create some test files
      vfs.writeFileSync('/file1.txt', 'content1');
      vfs.writeFileSync('/file2.txt', 'content2');

      // Re-create runtime to pick up new files
      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');

exec('ls /', (error, stdout, stderr) => {
  if (error) {
    console.log('error:', error.message);
    return;
  }
  console.log('files:', stdout);
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(consoleOutput.some(o => o.includes('file1.txt') || o.includes('files:'))).toBe(true);
    });

    it('should execute cat command', async () => {
      vfs.writeFileSync('/hello.txt', 'Hello, World!');

      // Re-create runtime to pick up new files
      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');

exec('cat /hello.txt', (error, stdout, stderr) => {
  if (error) {
    console.log('error:', error.message);
    return;
  }
  console.log('content:', stdout);
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(consoleOutput.some(o => o.includes('Hello, World!'))).toBe(true);
    });
  });

  describe('spawn', () => {
    it('should spawn echo command and emit exit', async () => {
      const code = `
const { spawn } = require('child_process');

const child = spawn('echo', ['Hello', 'World']);

child.on('close', (code) => {
  console.log('exit code:', code);
});

child.on('exit', (code) => {
  console.log('process exited with:', code);
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      // Check that the process completed successfully
      expect(consoleOutput.some(o => o.includes('exit code: 0') || o.includes('process exited with: 0'))).toBe(true);
    });
  });

  describe('shell features', () => {
    it('should support pipes', async () => {
      const code = `
const { exec } = require('child_process');

exec('echo "line1\\nline2\\nline3" | wc -l', (error, stdout, stderr) => {
  if (error) {
    console.log('error:', error.message);
    return;
  }
  console.log('lines:', stdout.trim());
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(consoleOutput.some(o => o.includes('3') || o.includes('lines:'))).toBe(true);
    });

    it('should support command chaining with &&', async () => {
      const code = `
const { exec } = require('child_process');

exec('echo "first" && echo "second"', (error, stdout, stderr) => {
  if (error) {
    console.log('error:', error.message);
    return;
  }
  console.log('output:', stdout);
});
      `;

      runtime.execute(code, '/test.js');

      // Wait for async execution
      await new Promise(resolve => setTimeout(resolve, 100));

      expect(consoleOutput.some(o => o.includes('first') && o.includes('second'))).toBe(true);
    });
  });

  describe('npm command', () => {
    it('should execute a script from package.json with npm run', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        version: '1.0.0',
        scripts: { hello: 'echo hello from npm' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm run hello', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('hello from npm'))).toBe(true);
    });

    it('should list available scripts when npm run has no args', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { build: 'echo building', dev: 'echo devving' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm run', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('build') && o.includes('dev'))).toBe(true);
    });

    it('should support npm start shorthand', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { start: 'echo started' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm start', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('started'))).toBe(true);
    });

    it('should support npm test shorthand', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { test: 'echo tested' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm test', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('tested'))).toBe(true);
    });

    it('should return error for missing script', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { build: 'echo build' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm run nonexistent', (error, stdout, stderr) => {
  console.log('STDERR:' + stderr);
  if (error) console.log('EXITCODE:' + error.code);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('Missing script') && o.includes('nonexistent'))).toBe(true);
      expect(consoleOutput.some(o => o.includes('EXITCODE:'))).toBe(true);
    });

    it('should return error when package.json is missing', async () => {
      // No package.json written to VFS

      const code = `
const { exec } = require('child_process');
exec('npm run build', (error, stdout, stderr) => {
  console.log('STDERR:' + stderr);
  if (error) console.log('FAILED');
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('no package.json'))).toBe(true);
      expect(consoleOutput.some(o => o.includes('FAILED'))).toBe(true);
    });

    it('should execute pre and post lifecycle scripts', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: {
          prebuild: 'echo pre',
          build: 'echo main',
          postbuild: 'echo post',
        },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm run build', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 300));

      const stdoutLine = consoleOutput.find(o => o.startsWith('STDOUT:'));
      expect(stdoutLine).toBeDefined();
      // All three should appear in stdout, in order
      const stdout = stdoutLine!;
      const preIdx = stdout.indexOf('pre');
      const mainIdx = stdout.indexOf('main');
      const postIdx = stdout.indexOf('post');
      expect(preIdx).toBeGreaterThanOrEqual(0);
      expect(mainIdx).toBeGreaterThan(preIdx);
      expect(postIdx).toBeGreaterThan(mainIdx);
    });

    it('should execute scripts with shell features', async () => {
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { combo: 'echo first && echo second' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm run combo', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('first') && o.includes('second'))).toBe(true);
    });

    it('should execute scripts that invoke node', async () => {
      vfs.writeFileSync('/script.js', 'console.log("node script ran");');
      vfs.writeFileSync('/package.json', JSON.stringify({
        name: 'test-app',
        scripts: { start: 'node /script.js' },
      }));

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('npm start', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 300));

      expect(consoleOutput.some(o => o.includes('node script ran'))).toBe(true);
    });

    it('should show help with npm --help', async () => {
      const code = `
const { exec } = require('child_process');
exec('npm --help', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('Usage: npm'))).toBe(true);
    });

    it('should return error for unknown subcommand', async () => {
      const code = `
const { exec } = require('child_process');
exec('npm foobar', (error, stdout, stderr) => {
  console.log('STDERR:' + stderr);
  if (error) console.log('FAILED');
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(consoleOutput.some(o => o.includes('Unknown command') && o.includes('foobar'))).toBe(true);
    });
  });

  describe('bin stubs', () => {
    it('should resolve commands from /node_modules/.bin/ via PATH', async () => {
      // Create a simple bin stub like npm install would
      vfs.mkdirSync('/node_modules/.bin', { recursive: true });
      vfs.writeFileSync('/node_modules/.bin/hello', 'node "/node_modules/hello/cli.js" "$@"\n');

      // Create the actual script
      vfs.mkdirSync('/node_modules/hello', { recursive: true });
      vfs.writeFileSync('/node_modules/hello/cli.js', 'console.log("hello from bin stub");');

      runtime = new Runtime(vfs, {
        onConsole: (method, args) => {
          consoleOutput.push(args.join(' '));
        },
      });

      const code = `
const { exec } = require('child_process');
exec('hello', (error, stdout, stderr) => {
  console.log('STDOUT:' + stdout);
  if (error) console.log('ERROR:' + error.message);
});
      `;

      runtime.execute(code, '/test.js');
      await new Promise(resolve => setTimeout(resolve, 500));

      const output = consoleOutput.join('\n');
      expect(output).toContain('hello from bin stub');
    });
  });
});
