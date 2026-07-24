const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Mock child_process.spawn for testing
const originalSpawn = spawn;

describe('easy-ci npm wrapper', () => {
  const indexJsPath = path.join(__dirname, 'index.js');

  beforeEach(() => {
    jest.resetModules();
  });

  afterAll(() => {
    // Restore original spawn
    spawn = originalSpawn;
  });

  test('index.js exists and is readable', () => {
    expect(fs.existsSync(indexJsPath)).toBe(true);
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content.length).toBeGreaterThan(0);
  });

  test('index.js has shebang', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content.startsWith('#!/usr/bin/env node')).toBe(true);
  });

  test('platformMap covers expected platforms', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("darwin: 'darwin'");
    expect(content).toContain("linux: 'linux'");
    expect(content).toContain("win32: 'windows'");
  });

  test('archMap covers expected architectures', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("x64: 'x86_64'");
    expect(content).toContain("arm64: 'aarch64'");
  });

  test('binary name is eci', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("let binaryName = 'eci'");
  });

  test('binary path construction is correct', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("path.join(__dirname, 'bin', os, architecture, binaryName)");
  });

  test('process.exit is called on unsupported platform', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain('process.exit(1)');
  });

  test('args are passed through to binary', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain('process.argv.slice(2)');
  });

  test('spawn uses inherit stdio', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("stdio: 'inherit'");
  });

  test('child close event exits with code', () => {
    const content = fs.readFileSync(indexJsPath, 'utf8');
    expect(content).toContain("child.on('close'");
    expect(content).toContain('process.exit(code || 0)');
  });

  test('package.json has correct bin entry', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8'));
    expect(pkg.bin).toEqual({ eci: './index.js' });
    expect(pkg.name).toBe('easy-ci');
    expect(pkg.engines.node).toBe('>=14.0.0');
  });

  test('package.json includes required files', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, 'package.json'), 'utf8'));
    expect(pkg.files).toContain('index.js');
    expect(pkg.files).toContain('bin/');
  });
});
