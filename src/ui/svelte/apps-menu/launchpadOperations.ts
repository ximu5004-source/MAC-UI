/** Coalesce queued placement requests and never run native placement calls concurrently. */
export class LatestPlacement<T> {
  private revision = 0;
  private queue: Promise<void> = Promise.resolve();
  constructor(private readonly apply: (value: T) => Promise<void>) {}

  request(value: T): Promise<void> {
    const revision = ++this.revision;
    this.queue = this.queue.catch(() => {}).then(async () => {
      if (revision === this.revision) await this.apply(value);
    });
    return this.queue;
  }

  async settled(): Promise<void> {
    let pending: Promise<void>;
    do { pending = this.queue; await pending; } while (pending !== this.queue);
  }
}

/** Preserve toggle parity while coalescing rapid requests into the latest intent. */
export class LatestToggle<T> {
  private revision = 0;
  private queue: Promise<void> = Promise.resolve();
  private desired: boolean | undefined;
  constructor(
    private readonly isVisible: () => Promise<boolean>,
    private readonly apply: (visible: boolean, input: T, isCurrent: () => boolean) => Promise<void>,
  ) {}

  toggle(input: T): Promise<void> {
    const revision = ++this.revision;
    this.queue = this.queue.catch(() => {}).then(async () => {
      this.desired ??= await this.isVisible();
      this.desired = !this.desired;
      if (revision === this.revision) await this.apply(this.desired, input, () => revision === this.revision);
    }).finally(() => {
      // Escape, focus loss and background clicks can hide the window independently.
      // The next non-overlapping trigger must query native visibility afresh.
      if (revision === this.revision) this.desired = undefined;
    });
    return this.queue;
  }
}
