import * as fs from 'fs';
import * as path from 'path';

export const VERSION_MARKER_FILE = '.agnix-lsp-version';
export const VERIFIED_MARKER_FILE = '.agnix-lsp-verified';

export interface VersionCheckDeps {
  readFileSync: (filePath: string, encoding: BufferEncoding) => string;
  writeFileSync: (filePath: string, data: string, encoding: BufferEncoding) => void;
}

const defaultDeps: VersionCheckDeps = {
  readFileSync: (p, enc) => fs.readFileSync(p, enc),
  writeFileSync: (p, data, enc) => fs.writeFileSync(p, data, enc),
};

export function readVersionMarker(
  storagePath: string,
  deps: VersionCheckDeps = defaultDeps
): string | null {
  const markerPath = path.join(storagePath, VERSION_MARKER_FILE);
  try {
    return deps.readFileSync(markerPath, 'utf-8').trim();
  } catch {
    return null;
  }
}

export function writeVersionMarker(
  storagePath: string,
  version: string,
  deps: VersionCheckDeps = defaultDeps
): void {
  const markerPath = path.join(storagePath, VERSION_MARKER_FILE);
  deps.writeFileSync(markerPath, version, 'utf-8');
}

export function readVerifiedMarker(
  storagePath: string,
  deps: VersionCheckDeps = defaultDeps
): boolean {
  const markerPath = path.join(storagePath, VERIFIED_MARKER_FILE);
  try {
    return deps.readFileSync(markerPath, 'utf-8').trim() === 'ok';
  } catch {
    return false;
  }
}

export function writeVerifiedMarker(
  storagePath: string,
  deps: VersionCheckDeps = defaultDeps
): void {
  const markerPath = path.join(storagePath, VERIFIED_MARKER_FILE);
  deps.writeFileSync(markerPath, 'ok', 'utf-8');
}

export function clearVerifiedMarker(
  storagePath: string,
  deps: VersionCheckDeps = defaultDeps
): void {
  const markerPath = path.join(storagePath, VERIFIED_MARKER_FILE);
  try {
    deps.writeFileSync(markerPath, '', 'utf-8');
  } catch {
    // Ignore if file doesn't exist
  }
}

export function isDownloadedBinary(
  lspPath: string,
  storagePath: string
): boolean {
  return lspPath.startsWith(storagePath);
}

export function buildReleaseUrl(
  repo: string,
  version: string,
  asset: string
): string {
  return `https://github.com/${repo}/releases/download/v${version}/${asset}`;
}

/**
 * Parse the version from `agnix-lsp --version` output.
 * Expected format: "agnix-lsp X.Y.Z"
 * Returns the version string or null if it can't be parsed.
 */
export function parseLspVersionOutput(output: string): string | null {
  const match = output.trim().match(/^agnix-lsp\s+(\d+\.\d+\.\d+)/);
  return match ? match[1] : null;
}
