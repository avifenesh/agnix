#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const os = require('os');

const GITHUB_REPO = 'avifenesh/agnix';
const VERSION = require('./package.json').version;

/**
 * Get platform-specific asset name and binary name.
 */
function getPlatformInfo() {
  const platform = os.platform();
  const arch = os.arch();

  const mapping = {
    'darwin-arm64': {
      asset: 'agnix-aarch64-apple-darwin.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'darwin-x64': {
      // x64 Mac uses ARM binary via Rosetta 2
      asset: 'agnix-aarch64-apple-darwin.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'linux-x64': {
      asset: 'agnix-x86_64-unknown-linux-gnu.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'linux-arm64': {
      // ARM Linux not yet available, try x64 with emulation
      asset: 'agnix-x86_64-unknown-linux-gnu.tar.gz',
      extractedName: 'agnix',
      binary: 'agnix-binary',
    },
    'win32-x64': {
      asset: 'agnix-x86_64-pc-windows-msvc.zip',
      extractedName: 'agnix.exe',
      binary: 'agnix-binary.exe',
    },
  };

  const key = `${platform}-${arch}`;
  const info = mapping[key];

  if (!info) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('Supported platforms: darwin-arm64, darwin-x64, linux-x64, win32-x64');
    process.exit(1);
  }

  return info;
}

/**
 * Download a file from URL, following redirects.
 */
function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);

    const request = https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          file.close();
          fs.unlinkSync(destPath);
          downloadFile(redirectUrl, destPath).then(resolve).catch(reject);
          return;
        }
      }

      if (response.statusCode !== 200) {
        file.close();
        reject(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }

      response.pipe(file);

      file.on('finish', () => {
        file.close();
        resolve();
      });
    });

    request.on('error', (err) => {
      file.close();
      fs.unlinkSync(destPath);
      reject(err);
    });

    file.on('error', (err) => {
      fs.unlinkSync(destPath);
      reject(err);
    });
  });
}

/**
 * Extract archive based on platform.
 */
function extractArchive(archivePath, destDir) {
  const platform = os.platform();

  if (platform === 'win32') {
    // Use PowerShell to extract zip
    execSync(
      `powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`,
      { stdio: 'inherit' }
    );
  } else {
    // Use tar for .tar.gz
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'inherit' });
  }
}

async function main() {
  const platformInfo = getPlatformInfo();
  const binDir = path.join(__dirname, 'bin');
  const binaryPath = path.join(binDir, platformInfo.binary);
  const extractedPath = path.join(binDir, platformInfo.extractedName);

  // Skip if binary already exists
  if (fs.existsSync(binaryPath)) {
    console.log('agnix binary already installed');
    return;
  }

  // Ensure bin directory exists
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/${platformInfo.asset}`;
  const archivePath = path.join(binDir, platformInfo.asset);

  console.log(`Downloading agnix v${VERSION} for ${os.platform()}-${os.arch()}...`);

  try {
    await downloadFile(downloadUrl, archivePath);
    console.log('Extracting...');
    extractArchive(archivePath, binDir);

    // Clean up archive
    fs.unlinkSync(archivePath);

    // Rename extracted binary to avoid conflict with wrapper script
    if (fs.existsSync(extractedPath) && extractedPath !== binaryPath) {
      fs.renameSync(extractedPath, binaryPath);
    }

    // Make binary executable on Unix
    if (os.platform() !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Verify binary exists
    if (!fs.existsSync(binaryPath)) {
      throw new Error('Binary not found after extraction');
    }

    console.log('agnix installed successfully');
  } catch (error) {
    console.error(`Failed to install agnix: ${error.message}`);
    console.error('');
    console.error('You can install manually:');
    console.error('  cargo install agnix-cli');
    process.exit(1);
  }
}

main();
