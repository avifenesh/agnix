export interface ClientLifecycleDelegate<TClient> {
  getClient(): TClient | undefined;
  setClient(client: TClient | undefined): void;
  isClientActive?(client: TClient): boolean;
  startClient(): Promise<TClient | undefined>;
  stopClient(client: TClient): Promise<void>;
}

/**
 * Serializes lifecycle transitions for the shared language client instance.
 *
 * A generation token plus desired enabled state prevents stale starts from
 * keeping an outdated client alive when newer operations supersede it.
 */
export class ClientLifecycleController<TClient> {
  private queue: Promise<void> = Promise.resolve();
  private generation = 0;
  private desiredEnabled = false;

  constructor(private readonly delegate: ClientLifecycleDelegate<TClient>) {}

  public start(): Promise<void> {
    this.desiredEnabled = true;
    const token = ++this.generation;
    return this.enqueue(() => this.runStart(token));
  }

  public stop(): Promise<void> {
    this.desiredEnabled = false;
    ++this.generation;
    return this.enqueue(() => this.runStop());
  }

  public restart(): Promise<void> {
    this.desiredEnabled = true;
    const token = ++this.generation;
    return this.enqueue(async () => {
      await this.runStop();
      if (!this.shouldStart(token)) {
        return;
      }
      await this.runStart(token);
    });
  }

  private enqueue(operation: () => Promise<void>): Promise<void> {
    const run = this.queue.then(operation, operation);
    this.queue = run.catch(() => undefined);
    return run;
  }

  private shouldStart(token: number): boolean {
    return this.desiredEnabled && token === this.generation;
  }

  private isClientActive(client: TClient): boolean {
    if (!this.delegate.isClientActive) {
      return true;
    }
    return this.delegate.isClientActive(client);
  }

  private async runStart(token: number): Promise<void> {
    if (!this.shouldStart(token)) {
      return;
    }

    const runningClient = this.delegate.getClient();
    if (runningClient) {
      if (this.isClientActive(runningClient)) {
        return;
      }
      await this.delegate.stopClient(runningClient);
      if (this.delegate.getClient() === runningClient) {
        this.delegate.setClient(undefined);
      }
      if (!this.shouldStart(token)) {
        return;
      }
    }

    const startedClient = await this.delegate.startClient();
    if (!startedClient) {
      return;
    }

    if (this.shouldStart(token)) {
      this.delegate.setClient(startedClient);
      return;
    }

    await this.delegate.stopClient(startedClient);
    if (this.delegate.getClient() === startedClient) {
      this.delegate.setClient(undefined);
    }
  }

  private async runStop(): Promise<void> {
    const runningClient = this.delegate.getClient();
    if (!runningClient) {
      return;
    }

    await this.delegate.stopClient(runningClient);
    if (this.delegate.getClient() === runningClient) {
      this.delegate.setClient(undefined);
    }
  }
}
