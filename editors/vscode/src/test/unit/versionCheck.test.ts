import * as assert from 'assert';
import * as path from 'path';
import {
  readVersionMarker,
  writeVersionMarker,
  readVerifiedMarker,
  writeVerifiedMarker,
  clearVerifiedMarker,
  isDownloadedBinary,
  buildReleaseUrl,
  parseLspVersionOutput,
  VERSION_MARKER_FILE,
  VERIFIED_MARKER_FILE,
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
      [path.join('/storage', VERSION_MARKER_FILE)]: '0.9.1',
    });
    const result = readVersionMarker('/storage', deps);
    assert.strictEqual(result, '0.9.1');
  });

  it('trims whitespace from marker content', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERSION_MARKER_FILE)]: '  0.9.1\n',
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
      deps.written[path.join('/storage', VERSION_MARKER_FILE)],
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

describe('parseLspVersionOutput', () => {
  it('parses standard version output', () => {
    assert.strictEqual(parseLspVersionOutput('agnix-lsp 0.9.2'), '0.9.2');
  });

  it('parses output with trailing newline', () => {
    assert.strictEqual(parseLspVersionOutput('agnix-lsp 1.0.0\n'), '1.0.0');
  });

  it('returns null for empty output', () => {
    assert.strictEqual(parseLspVersionOutput(''), null);
  });

  it('returns null for unrecognized output', () => {
    assert.strictEqual(parseLspVersionOutput('some other program 1.0'), null);
  });

  it('returns null for LSP JSON-RPC output (old binary)', () => {
    assert.strictEqual(
      parseLspVersionOutput('Content-Length: 123\r\n'),
      null
    );
  });
});

describe('readVerifiedMarker', () => {
  it('returns false when marker file does not exist', () => {
    const deps = createMockDeps();
    assert.strictEqual(readVerifiedMarker('/storage', deps), false);
  });

  it('returns true when marker contains ok', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERIFIED_MARKER_FILE)]: 'ok',
    });
    assert.strictEqual(readVerifiedMarker('/storage', deps), true);
  });

  it('returns true when marker has trailing whitespace', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERIFIED_MARKER_FILE)]: '  ok\n',
    });
    assert.strictEqual(readVerifiedMarker('/storage', deps), true);
  });

  it('returns false when marker has wrong content', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERIFIED_MARKER_FILE)]: 'bad',
    });
    assert.strictEqual(readVerifiedMarker('/storage', deps), false);
  });

  it('returns false when marker is empty', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERIFIED_MARKER_FILE)]: '',
    });
    assert.strictEqual(readVerifiedMarker('/storage', deps), false);
  });
});

describe('writeVerifiedMarker', () => {
  it('writes ok to the correct path', () => {
    const deps = createMockDeps();
    writeVerifiedMarker('/storage', deps);
    assert.strictEqual(
      deps.written[path.join('/storage', VERIFIED_MARKER_FILE)],
      'ok'
    );
  });
});

describe('clearVerifiedMarker', () => {
  it('writes empty string to clear the marker', () => {
    const deps = createMockDeps({
      [path.join('/storage', VERIFIED_MARKER_FILE)]: 'ok',
    });
    clearVerifiedMarker('/storage', deps);
    assert.strictEqual(
      deps.written[path.join('/storage', VERIFIED_MARKER_FILE)],
      ''
    );
  });
});

describe('verified marker roundtrip', () => {
  it('write then read returns true', () => {
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

    assert.strictEqual(readVerifiedMarker('/s', deps), false);
    writeVerifiedMarker('/s', deps);
    assert.strictEqual(readVerifiedMarker('/s', deps), true);
  });

  it('clear after write returns false', () => {
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

    writeVerifiedMarker('/s', deps);
    assert.strictEqual(readVerifiedMarker('/s', deps), true);
    clearVerifiedMarker('/s', deps);
    assert.strictEqual(readVerifiedMarker('/s', deps), false);
  });
});
