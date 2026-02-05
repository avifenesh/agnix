import * as fs from 'fs';
import * as https from 'https';

type WritableFile = {
  close: () => void;
  on: (event: string, listener: (...args: unknown[]) => void) => unknown;
};

type ResponseLike = {
  statusCode?: number;
  headers: Record<string, string | string[] | undefined>;
  pipe: (dest: WritableFile) => unknown;
  on: (event: string, listener: (...args: unknown[]) => void) => unknown;
  resume?: () => void;
  destroy?: () => void;
};

type RequestLike = {
  on: (event: string, listener: (...args: unknown[]) => void) => unknown;
  destroy: () => void;
};

export interface DownloadFileDeps {
  createWriteStream: (path: string) => WritableFile;
  unlinkSync: (path: string) => void;
  get: (url: string, cb: (response: ResponseLike) => void) => RequestLike;
}

const defaultDeps: DownloadFileDeps = {
  createWriteStream: (filePath) => fs.createWriteStream(filePath),
  unlinkSync: (filePath) => fs.unlinkSync(filePath),
  get: (url, cb) => https.get(url, cb as (response: unknown) => void) as unknown as RequestLike,
};

function toError(value: unknown): Error {
  if (value instanceof Error) {
    return value;
  }
  return new Error(String(value));
}

/**
 * Download a file from URL, following redirects.
 */
export function downloadFile(
  url: string,
  destPath: string,
  deps: DownloadFileDeps = defaultDeps
): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = deps.createWriteStream(destPath);
    let request: RequestLike | null = null;
    let response: ResponseLike | null = null;
    let settled = false;

    const closeFile = () => {
      try {
        file.close();
      } catch {
        // Error ignored during cleanup
      }
    };

    const cleanupTempFile = () => {
      try {
        deps.unlinkSync(destPath);
      } catch {
        // Error ignored during cleanup
      }
    };

    const resolveOnce = () => {
      if (settled) {
        return;
      }
      settled = true;
      resolve();
    };

    const rejectOnce = (error: Error) => {
      if (settled) {
        return;
      }
      settled = true;
      reject(error);
    };

    const fail = (error: Error) => {
      closeFile();
      if (request) {
        request.destroy();
      }
      if (response && typeof response.destroy === 'function') {
        response.destroy();
      }
      cleanupTempFile();
      rejectOnce(error);
    };

    file.on('error', (err) => {
      fail(toError(err));
    });

    file.on('finish', () => {
      closeFile();
      resolveOnce();
    });

    request = deps.get(url, (res) => {
      response = res;

      // Handle redirects (GitHub releases use them)
      if (res.statusCode === 302 || res.statusCode === 301) {
        const redirect = res.headers.location;
        const redirectUrl = Array.isArray(redirect) ? redirect[0] : redirect;
        if (redirectUrl) {
          closeFile();
          cleanupTempFile();
          if (typeof res.resume === 'function') {
            res.resume();
          }
          downloadFile(redirectUrl, destPath, deps).then(resolveOnce).catch((err) => {
            rejectOnce(toError(err));
          });
          return;
        }
      }

      if (res.statusCode !== 200) {
        if (typeof res.resume === 'function') {
          res.resume();
        }
        fail(new Error(`Download failed with status ${res.statusCode}`));
        return;
      }

      res.on('error', (err) => {
        fail(toError(err));
      });

      res.pipe(file);
    });

    request.on('error', (err) => {
      fail(toError(err));
    });
  });
}
