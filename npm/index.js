#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const { platform, arch } = process;

const platformMap = {
  darwin: 'darwin',
  linux: 'linux',
  win32: 'windows',
};

const archMap = {
  x64: 'x86_64',
  arm64: 'aarch64',
};

const os = platformMap[platform];
const architecture = archMap[arch];

if (!os || !architecture) {
  console.error(`Unsupported platform: ${platform}/${arch}`);
  console.error('easy-ci supports: darwin/linux/windows on x64/arm64');
  process.exit(1);
}

let binaryName = 'eci';
if (platform === 'win32') {
  binaryName += '.exe';
}

const binaryPath = path.join(__dirname, 'bin', os, architecture, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`Binary not found: ${binaryPath}`);
  console.error('This platform may not be supported yet.');
  process.exit(1);
}

const args = process.argv.slice(2);
const child = spawn(binaryPath, args, { stdio: 'inherit' });

child.on('close', (code) => {
  process.exit(code || 0);
});
