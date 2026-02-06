import * as assert from 'assert';
import {
  readVersionMarker,
  writeVersionMarker,
  isDownloadedBinary,
  buildReleaseUrl,
  VERSION_MARKER_FILE,
  type VersionCheckDeps,
} from '../../version-check';

function createMockDeps(
  files: Record<string, string> = {}
): VersionCheckDeps & { written: Record<string, string> } {
  const written: Record<string, string> = {};
  return {
    written,
    readFileSync: (filePath: string) => {
      if (filePath in files) {
        return files[filePath];
      }
      throw new Error('ENOENT: no such file');
    },
    writeFileSync: (filePath: string, data: string) => {
      written[filePath] = data;
    },
  };
}

describe('readVersionMarker', () => {
  it('returns null when marker file does not exist', () => {
    const deps = createMockDeps();
    const result = readVersionMarker('/storage', deps);
    assert.strictEqual(result, null);
  });

  it('returns version string when marker exists', () => {
    const deps = createMockDeps({
      [`/storage/${VERSION_MARKER_FILE}`]: '0.9.1',
    });
    const result = readVersionMarker('/storage', deps);
    assert.strictEqual(result, '0.9.1');
  });

  it('trims whitespace from marker content', () => {
    const deps = createMockDeps({
      [`/storage/${VERSION_MARKER_FILE}`]: '  0.9.1\n',
    });
    const result = readVersionMarker('/storage', deps);
    assert.strictEqual(result, '0.9.1');
  });
});

describe('writeVersionMarker', () => {
  it('writes version to the correct path', () => {
    const deps = createMockDeps();
    writeVersionMarker('/storage', '0.9.1', deps);
    assert.strictEqual(
      deps.written[`/storage/${VERSION_MARKER_FILE}`],
      '0.9.1'
    );
  });
});

describe('readVersionMarker + writeVersionMarker roundtrip', () => {
  it('read returns what was written', () => {
    const files: Record<string, string> = {};
    const deps: VersionCheckDeps = {
      readFileSync: (filePath: string) => {
        if (filePath in files) {
          return files[filePath];
        }
        throw new Error('ENOENT');
      },
      writeFileSync: (filePath: string, data: string) => {
        files[filePath] = data;
      },
    };

    assert.strictEqual(readVersionMarker('/s', deps), null);
    writeVersionMarker('/s', '1.2.3', deps);
    assert.strictEqual(readVersionMarker('/s', deps), '1.2.3');
  });
});

describe('isDownloadedBinary', () => {
  it('returns true when path is inside storage', () => {
    assert.strictEqual(
      isDownloadedBinary('/home/user/.vscode/globalStorage/agnix-lsp', '/home/user/.vscode/globalStorage'),
      true
    );
  });

  it('returns false for user-configured path', () => {
    assert.strictEqual(
      isDownloadedBinary('/usr/local/bin/agnix-lsp', '/home/user/.vscode/globalStorage'),
      false
    );
  });

  it('returns false for PATH-resolved binary', () => {
    assert.strictEqual(
      isDownloadedBinary('/opt/homebrew/bin/agnix-lsp', '/home/user/.vscode/globalStorage'),
      false
    );
  });
});

describe('buildReleaseUrl', () => {
  it('constructs correct versioned URL', () => {
    const url = buildReleaseUrl(
      'avifenesh/agnix',
      '0.9.1',
      'agnix-lsp-aarch64-apple-darwin.tar.gz'
    );
    assert.strictEqual(
      url,
      'https://github.com/avifenesh/agnix/releases/download/v0.9.1/agnix-lsp-aarch64-apple-darwin.tar.gz'
    );
  });

  it('handles different versions and assets', () => {
    const url = buildReleaseUrl(
      'avifenesh/agnix',
      '1.0.0',
      'agnix-lsp-x86_64-pc-windows-msvc.zip'
    );
    assert.strictEqual(
      url,
      'https://github.com/avifenesh/agnix/releases/download/v1.0.0/agnix-lsp-x86_64-pc-windows-msvc.zip'
    );
  });
});
