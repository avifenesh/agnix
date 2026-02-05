/**
 * agnix - Node.js API
 *
 * Programmatic access to the agnix linter.
 */

const { spawn, spawnSync } = require('child_process');
const path = require('path');
const os = require('os');

const binDir = path.join(__dirname, 'bin');
const binaryName = os.platform() === 'win32' ? 'agnix-binary.exe' : 'agnix-binary';
const binaryPath = path.join(binDir, binaryName);

/**
 * Run agnix synchronously and return the result.
 *
 * @param {string[]} args - Command line arguments
 * @param {object} options - Options passed to spawnSync
 * @returns {{ stdout: string, stderr: string, exitCode: number }}
 */
function runSync(args = [], options = {}) {
  const result = spawnSync(binaryPath, args, {
    encoding: 'utf-8',
    ...options,
  });

  if (result.error) {
    throw result.error;
  }

  return {
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    exitCode: result.status || 0,
  };
}

/**
 * Run agnix asynchronously and return a promise.
 *
 * @param {string[]} args - Command line arguments
 * @param {object} options - Options passed to spawn
 * @returns {Promise<{ stdout: string, stderr: string, exitCode: number }>}
 */
function run(args = [], options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(binaryPath, args, options);
    let stdout = '';
    let stderr = '';

    if (child.stdout) {
      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });
    }

    if (child.stderr) {
      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });
    }

    child.on('error', reject);

    child.on('close', (code) => {
      resolve({
        stdout,
        stderr,
        exitCode: code || 0,
      });
    });
  });
}

/**
 * Lint files and return diagnostics as JSON.
 *
 * @param {string} target - Path to file or directory to lint
 * @param {object} options - Lint options
 * @param {string} options.target - Target tool (ClaudeCode, Cursor, etc.)
 * @param {string} options.format - Output format (json, sarif)
 * @returns {Promise<object>} - Parsed JSON diagnostics
 */
async function lint(target, options = {}) {
  const args = ['--format', 'json'];

  if (options.target) {
    args.push('--target', options.target);
  }

  args.push(target);

  const result = await run(args);

  try {
    return JSON.parse(result.stdout);
  } catch {
    return {
      files: [],
      summary: { errors: 0, warnings: 0, fixable: 0 },
      raw: result.stdout,
    };
  }
}

/**
 * Get agnix version.
 *
 * @returns {string} - Version string
 */
function version() {
  const result = runSync(['--version']);
  return result.stdout.trim();
}

module.exports = {
  run,
  runSync,
  lint,
  version,
  binaryPath,
};
