import * as assert from 'assert';
import { ClientLifecycleController } from '../../clientLifecycle';

interface MockClient {
  id: number;
}

function createDeferred(): { promise: Promise<void>; resolve: () => void } {
  let resolvePromise: (() => void) | undefined;
  const promise = new Promise<void>((resolve) => {
    resolvePromise = resolve;
  });

  return {
    promise,
    resolve: () => {
      if (resolvePromise) {
        resolvePromise();
      }
    },
  };
}

describe('ClientLifecycleController', () => {
  it('serializes lifecycle operations and avoids overlap', async () => {
    let currentClient: MockClient | undefined;
    let activeOps = 0;
    let maxConcurrentOps = 0;
    let startCalls = 0;
    let stopCalls = 0;
    const events: string[] = [];
    const firstStartGate = createDeferred();

    const controller = new ClientLifecycleController<MockClient>({
      getClient: () => currentClient,
      setClient: (nextClient) => {
        currentClient = nextClient;
      },
      startClient: async () => {
        activeOps += 1;
        maxConcurrentOps = Math.max(maxConcurrentOps, activeOps);
        startCalls += 1;
        const client: MockClient = { id: startCalls };
        events.push(`start-${client.id}-begin`);

        if (client.id === 1) {
          await firstStartGate.promise;
        }

        currentClient = client;
        events.push(`start-${client.id}-end`);
        activeOps -= 1;
        return client;
      },
      stopClient: async (runningClient) => {
        activeOps += 1;
        maxConcurrentOps = Math.max(maxConcurrentOps, activeOps);
        stopCalls += 1;
        events.push(`stop-${runningClient.id}-begin`);
        currentClient = undefined;
        events.push(`stop-${runningClient.id}-end`);
        activeOps -= 1;
      },
    });

    const firstStart = controller.start();
    await Promise.resolve();
    const restart = controller.restart();

    await Promise.resolve();
    assert.deepStrictEqual(events, ['start-1-begin']);

    firstStartGate.resolve();
    await Promise.all([firstStart, restart]);

    assert.strictEqual(maxConcurrentOps, 1);
    assert.strictEqual(startCalls, 2);
    assert.strictEqual(stopCalls, 1);
    assert.deepStrictEqual(events, [
      'start-1-begin',
      'start-1-end',
      'stop-1-begin',
      'stop-1-end',
      'start-2-begin',
      'start-2-end',
    ]);
    assert.deepStrictEqual(currentClient, { id: 2 });
  });

  it('stops stale start when disabled during startup', async () => {
    let currentClient: MockClient | undefined;
    let startCalls = 0;
    let stopCalls = 0;
    const events: string[] = [];
    const startGate = createDeferred();

    const controller = new ClientLifecycleController<MockClient>({
      getClient: () => currentClient,
      setClient: (nextClient) => {
        currentClient = nextClient;
      },
      startClient: async () => {
        startCalls += 1;
        const client: MockClient = { id: startCalls };
        events.push('start-begin');
        await startGate.promise;
        currentClient = client;
        events.push('start-end');
        return client;
      },
      stopClient: async (runningClient) => {
        stopCalls += 1;
        events.push(`stop-${runningClient.id}`);
        currentClient = undefined;
      },
    });

    const start = controller.start();
    await Promise.resolve();
    const stop = controller.stop();

    startGate.resolve();
    await Promise.all([start, stop]);

    assert.strictEqual(startCalls, 1);
    assert.strictEqual(stopCalls, 1);
    assert.strictEqual(currentClient, undefined);
    assert.deepStrictEqual(events, ['start-begin', 'start-end', 'stop-1']);
  });

  it('replaces stale inactive client references before starting', async () => {
    let currentClient: MockClient | undefined = { id: 1 };
    let startCalls = 0;
    let stopCalls = 0;
    const stoppedClients: number[] = [];

    const controller = new ClientLifecycleController<MockClient>({
      getClient: () => currentClient,
      setClient: (nextClient) => {
        currentClient = nextClient;
      },
      isClientActive: (runningClient) => runningClient.id !== 1,
      startClient: async () => {
        startCalls += 1;
        const client: MockClient = { id: 2 };
        currentClient = client;
        return client;
      },
      stopClient: async (runningClient) => {
        stopCalls += 1;
        stoppedClients.push(runningClient.id);
        currentClient = undefined;
      },
    });

    await controller.start();

    assert.strictEqual(startCalls, 1);
    assert.strictEqual(stopCalls, 1);
    assert.deepStrictEqual(stoppedClients, [1]);
    assert.deepStrictEqual(currentClient, { id: 2 });
  });
});
