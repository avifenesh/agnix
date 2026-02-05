import * as fs from 'fs';
import * as https from 'https';

type WritableFile = {
  close: () => void;
  on: (event: string, listener: (...args: unknown[]) => void) => unknown;
  emit?: (event: string, ...args: unknown[]) => boolean;
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
  get: (url, cb) =>
    https.get(url, (response) => cb(response as ResponseLike)) as RequestLike,
};

const MAX_REDIRECTS = 10;

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
  deps: DownloadFileDeps = defaultDeps,
  redirectCount = 0
): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = deps.createWriteStream(destPath);
    let request: RequestLike | null = null;
    let response: ResponseLike | null = null;
    let settled = false;
    let fileHandlersActive = true;
    let followingRedirect = false;

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

    const disableFileHandlers = () => {
      fileHandlersActive = false;
    };

    const fail = (error: Error) => {
      disableFileHandlers();
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

    const followRedirect = (redirectUrl: string) => {
      if (redirectCount >= MAX_REDIRECTS) {
        fail(new Error('Too many redirects'));
        return;
      }

      followingRedirect = true;
      disableFileHandlers();
      closeFile();
      cleanupTempFile();

      if (request) {
        request.destroy();
      }

      if (response && typeof response.resume === 'function') {
        response.resume();
      }

      if (response && typeof response.destroy === 'function') {
        response.destroy();
      }

      const nextUrl = new URL(redirectUrl, url).href;
      downloadFile(nextUrl, destPath, deps, redirectCount + 1)
        .then(resolveOnce)
        .catch((err) => {
          rejectOnce(toError(err));
        });
    };

    file.on('error', (err) => {
      if (!fileHandlersActive) {
        return;
      }
      fail(toError(err));
    });

    file.on('finish', () => {
      if (!fileHandlersActive) {
        return;
      }
      disableFileHandlers();
      closeFile();
      resolveOnce();
    });

    request = deps.get(url, (res) => {
      response = res;

      res.on('error', (err) => {
        if (followingRedirect) {
          return;
        }
        fail(toError(err));
      });

      // Handle redirects (GitHub releases use them)
      if (res.statusCode === 302 || res.statusCode === 301) {
        const redirect = res.headers.location;
        const redirectUrl = Array.isArray(redirect)
          ? (redirect.length > 0 ? redirect[0] : undefined)
          : redirect;
        if (redirectUrl) {
          followRedirect(redirectUrl);
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

      res.pipe(file);
    });

    request.on('error', (err) => {
      if (followingRedirect) {
        return;
      }
      fail(toError(err));
    });
  });
}
