#!/usr/bin/env node

/**
 * Tests for agnix npm package.
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const assert = require('assert');

const binDir = path.join(__dirname, '..', 'bin');
const binaryName = os.platform() === 'win32' ? 'agnix.exe' : 'agnix';
const binaryPath = path.join(binDir, binaryName);

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`  PASS: ${name}`);
    passed++;
  } catch (err) {
    console.log(`  FAIL: ${name}`);
    console.log(`        ${err.message}`);
    failed++;
  }
}

console.log('agnix npm package tests\n');

// Test 1: Binary exists after install
test('binary exists', () => {
  assert.ok(fs.existsSync(binaryPath), `Binary not found at ${binaryPath}`);
});

// Test 2: Binary is executable
test('binary is executable', () => {
  try {
    fs.accessSync(binaryPath, fs.constants.X_OK);
  } catch {
    // On Windows, X_OK check doesn't work the same way
    if (os.platform() !== 'win32') {
      throw new Error('Binary is not executable');
    }
  }
});

// Test 3: Version command works
test('--version works', () => {
  const agnix = require('..');
  const result = agnix.runSync(['--version']);
  assert.strictEqual(result.exitCode, 0, 'Exit code should be 0');
  assert.ok(result.stdout.includes('agnix'), 'Output should contain "agnix"');
});

// Test 4: Help command works
test('--help works', () => {
  const agnix = require('..');
  const result = agnix.runSync(['--help']);
  assert.strictEqual(result.exitCode, 0, 'Exit code should be 0');
  assert.ok(result.stdout.includes('Usage'), 'Output should contain "Usage"');
});

// Test 5: JSON output format
test('JSON output format', () => {
  const agnix = require('..');
  // Create a temp file to lint
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-test-'));
  const testFile = path.join(tempDir, 'CLAUDE.md');
  fs.writeFileSync(testFile, '# CLAUDE.md\n\nTest content.\n');

  try {
    const result = agnix.runSync(['--format', 'json', testFile]);
    // Should be valid JSON even if no errors
    const parsed = JSON.parse(result.stdout);
    assert.ok(parsed.hasOwnProperty('files'), 'Output should have files property');
    assert.ok(parsed.hasOwnProperty('summary'), 'Output should have summary property');
  } finally {
    fs.unlinkSync(testFile);
    fs.rmdirSync(tempDir);
  }
});

// Test 6: Node.js API version() function
test('API version() function', () => {
  const agnix = require('..');
  const ver = agnix.version();
  assert.ok(ver.includes('agnix'), 'Version should include "agnix"');
});

// Test 7: Node.js API lint() function
test('API lint() function', async () => {
  const agnix = require('..');
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agnix-test-'));
  const testFile = path.join(tempDir, 'CLAUDE.md');
  fs.writeFileSync(testFile, '# CLAUDE.md\n\nTest content.\n');

  try {
    const result = await agnix.lint(testFile);
    assert.ok(result.hasOwnProperty('files') || result.hasOwnProperty('raw'), 'Result should have files or raw');
  } finally {
    fs.unlinkSync(testFile);
    fs.rmdirSync(tempDir);
  }
});

// Summary
console.log(`\nResults: ${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
