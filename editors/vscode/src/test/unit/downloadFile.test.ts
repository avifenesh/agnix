import * as assert from 'assert';
import { EventEmitter } from 'events';
import { downloadFile, type DownloadFileDeps } from '../../download-file';

class FakeWriteStream extends EventEmitter {
  public closeCalls = 0;

  close(): void {
    this.closeCalls += 1;
  }
}

class FakeResponse extends EventEmitter {
  public statusCode = 200;
  public headers: Record<string, string | string[] | undefined> = {};
  private readonly onPipe: (dest: any) => void;

  constructor(onPipe: (dest: any) => void) {
    super();
    this.onPipe = onPipe;
  }

  pipe(dest: any): void {
    this.onPipe(dest);
  }

  resume(): void {}

  destroy(): void {}
}

class FakeRequest extends EventEmitter {
  public destroyed = false;

  destroy(): void {
    this.destroyed = true;
  }
}

describe('downloadFile', () => {
  it('cleans up temp file and rejects on non-200 responses', async () => {
    const writeStream = new FakeWriteStream();
    const request = new FakeRequest();
    const unlinkedPaths: string[] = [];

    const deps: DownloadFileDeps = {
      createWriteStream: () => writeStream,
      unlinkSync: (targetPath) => {
        unlinkedPaths.push(targetPath);
      },
      get: (_url, cb) => {
        const response = new FakeResponse(() => {
          // No pipe on non-200.
        });
        response.statusCode = 500;
        setImmediate(() => cb(response));
        return request;
      },
    };

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /status 500/
    );

    assert.ok(writeStream.closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
  });

  it('cleans up temp file and rejects on pipe/write stream error', async () => {
    const writeStream = new FakeWriteStream();
    const request = new FakeRequest();
    const unlinkedPaths: string[] = [];

    const deps: DownloadFileDeps = {
      createWriteStream: () => writeStream,
      unlinkSync: (targetPath) => {
        unlinkedPaths.push(targetPath);
      },
      get: (_url, cb) => {
        const response = new FakeResponse((dest) => {
          setImmediate(() => {
            dest.emit('error', new Error('disk full'));
          });
        });
        setImmediate(() => cb(response));
        return request;
      },
    };

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /disk full/
    );

    assert.ok(writeStream.closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
  });

  it('resolves successfully and does not delete file when download completes', async () => {
    const writeStream = new FakeWriteStream();
    const request = new FakeRequest();
    const unlinkedPaths: string[] = [];

    const deps: DownloadFileDeps = {
      createWriteStream: () => writeStream,
      unlinkSync: (targetPath) => {
        unlinkedPaths.push(targetPath);
      },
      get: (_url, cb) => {
        const response = new FakeResponse((dest) => {
          setImmediate(() => {
            dest.emit('finish');
          });
        });
        setImmediate(() => cb(response));
        return request;
      },
    };

    await downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps);

    assert.ok(writeStream.closeCalls > 0, 'expected write stream to be closed on success');
    assert.deepStrictEqual(unlinkedPaths, []);
    assert.strictEqual(request.destroyed, false);
  });
});
