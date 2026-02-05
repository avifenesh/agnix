import * as assert from 'assert';
import { EventEmitter } from 'events';
import { downloadFile, type DownloadFileDeps } from '../../download-file';

type PipeDestination = {
  emit?: (event: string, ...args: unknown[]) => boolean;
};

class FakeWriteStream extends EventEmitter {
  public closeCalls = 0;

  close(): void {
    this.closeCalls += 1;
  }
}

class FakeResponse extends EventEmitter {
  public statusCode = 200;
  public headers: Record<string, string | string[] | undefined> = {};
  public resumed = false;
  public destroyed = false;
  private readonly onPipe: (dest: PipeDestination) => void;

  constructor(onPipe: (dest: PipeDestination) => void) {
    super();
    this.onPipe = onPipe;
  }

  pipe(dest: PipeDestination): void {
    this.onPipe(dest);
  }

  resume(): void {
    this.resumed = true;
  }

  destroy(): void {
    this.destroyed = true;
  }
}

class FakeRequest extends EventEmitter {
  public destroyed = false;

  destroy(): void {
    this.destroyed = true;
  }
}

function createDeps(get: DownloadFileDeps['get']): {
  deps: DownloadFileDeps;
  writeStreams: FakeWriteStream[];
  unlinkedPaths: string[];
} {
  const writeStreams: FakeWriteStream[] = [];
  const unlinkedPaths: string[] = [];

  return {
    writeStreams,
    unlinkedPaths,
    deps: {
      createWriteStream: () => {
        const writeStream = new FakeWriteStream();
        writeStreams.push(writeStream);
        return writeStream;
      },
      unlinkSync: (targetPath) => {
        unlinkedPaths.push(targetPath);
      },
      get,
    },
  };
}

describe('downloadFile', () => {
  it('cleans up temp file and rejects on non-200 responses', async () => {
    const request = new FakeRequest();
    const response = new FakeResponse(() => {
      // No pipe on non-200.
    });
    response.statusCode = 500;

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      (_url, cb) => {
        setImmediate(() => cb(response));
        return request;
      }
    );

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /status 500/
    );

    assert.ok(writeStreams[0].closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
    assert.strictEqual(response.resumed, true, 'expected HTTP response to be resumed');
    assert.strictEqual(response.destroyed, true, 'expected HTTP response to be destroyed');
  });

  it('cleans up temp file and rejects on pipe/write stream error', async () => {
    const request = new FakeRequest();
    const response = new FakeResponse((dest) => {
      setImmediate(() => {
        if (dest.emit) {
          dest.emit('error', new Error('disk full'));
        }
      });
    });

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      (_url, cb) => {
        setImmediate(() => cb(response));
        return request;
      }
    );

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /disk full/
    );

    assert.ok(writeStreams[0].closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
    assert.strictEqual(response.destroyed, true, 'expected HTTP response to be destroyed');
  });

  it('cleans up temp file and rejects on request-level errors', async () => {
    const request = new FakeRequest();

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      () => {
        setImmediate(() => {
          request.emit('error', new Error('network down'));
        });
        return request;
      }
    );

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /network down/
    );

    assert.ok(writeStreams[0].closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
  });

  it('cleans up temp file and rejects on response stream errors', async () => {
    const request = new FakeRequest();
    const response = new FakeResponse(() => {
      // No file events for this path.
    });

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      (_url, cb) => {
        setImmediate(() => {
          cb(response);
          response.emit('error', new Error('socket reset'));
        });
        return request;
      }
    );

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /socket reset/
    );

    assert.ok(writeStreams[0].closeCalls > 0, 'expected write stream to be closed');
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
    assert.strictEqual(request.destroyed, true, 'expected HTTP request to be destroyed');
    assert.strictEqual(response.destroyed, true, 'expected HTTP response to be destroyed');
  });

  it('follows redirects using relative locations and succeeds', async () => {
    const requests: FakeRequest[] = [];
    const responses: FakeResponse[] = [];
    const calledUrls: string[] = [];
    const originalUrl = 'https://example.com/releases/latest/download/archive.tar.gz';

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      (url, cb) => {
        calledUrls.push(url);
        const request = new FakeRequest();
        requests.push(request);

        if (calledUrls.length === 1) {
          const response = new FakeResponse(() => {
            // Redirect response does not pipe content.
          });
          response.statusCode = 302;
          response.headers.location = '/archive.tar.gz';
          responses.push(response);
          setImmediate(() => cb(response));
          return request;
        }

        const response = new FakeResponse((dest) => {
          setImmediate(() => {
            if (dest.emit) {
              dest.emit('finish');
            }
          });
        });
        responses.push(response);
        setImmediate(() => cb(response));
        return request;
      }
    );

    await downloadFile(originalUrl, '/tmp/archive.tar.gz', deps);

    assert.deepStrictEqual(calledUrls, [
      originalUrl,
      'https://example.com/archive.tar.gz',
    ]);
    assert.strictEqual(requests[0].destroyed, true, 'expected first request to be destroyed on redirect');
    assert.strictEqual(responses[0].resumed, true, 'expected first response to be resumed on redirect');
    assert.strictEqual(responses[0].destroyed, true, 'expected first response to be destroyed on redirect');
    assert.ok(
      writeStreams.some((stream) => stream.closeCalls > 0),
      'expected stream close on redirect and success'
    );
    assert.deepStrictEqual(unlinkedPaths, ['/tmp/archive.tar.gz']);
  });

  it('fails after too many redirects', async () => {
    const requests: FakeRequest[] = [];
    let callCount = 0;

    const { deps, unlinkedPaths } = createDeps(
      (_url, cb) => {
        callCount += 1;
        const request = new FakeRequest();
        requests.push(request);
        const response = new FakeResponse(() => {
          // Redirect response does not pipe content.
        });
        response.statusCode = 302;
        response.headers.location = '/loop';
        setImmediate(() => cb(response));
        return request;
      }
    );

    await assert.rejects(
      downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps),
      /Too many redirects/
    );

    assert.strictEqual(callCount, 11, 'expected redirect limit to stop recursion');
    assert.ok(requests.every((req) => req.destroyed), 'expected all redirect requests to be destroyed');
    assert.ok(unlinkedPaths.length >= 1, 'expected temp file cleanup during redirect chain');
  });

  it('resolves successfully and does not delete file when download completes', async () => {
    const request = new FakeRequest();
    const response = new FakeResponse((dest) => {
      setImmediate(() => {
        if (dest.emit) {
          dest.emit('finish');
        }
      });
    });

    const { deps, writeStreams, unlinkedPaths } = createDeps(
      (_url, cb) => {
        setImmediate(() => cb(response));
        return request;
      }
    );

    await downloadFile('https://example.com/archive.tar.gz', '/tmp/archive.tar.gz', deps);

    assert.ok(writeStreams[0].closeCalls > 0, 'expected write stream to be closed on success');
    assert.deepStrictEqual(unlinkedPaths, []);
    assert.strictEqual(request.destroyed, false);
    assert.strictEqual(response.destroyed, false);
  });
});
