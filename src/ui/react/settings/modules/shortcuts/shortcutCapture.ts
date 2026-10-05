export type ShortcutCaptureResult = string[] | null;
type Unlisten = () => void | Promise<void>;

export interface ShortcutCapturePorts {
  listen(onFinished: (keys: ShortcutCaptureResult) => void): Promise<Unlisten>;
  request(): Promise<unknown>;
}

export interface ShortcutCapture {
  /** Resolves once the listener is installed and the capture request is sent. */
  ready: Promise<void>;
  cancel(): void;
}

let captureEventId = 0;

/** Each request must ignore completion/cancellation events from older sessions. */
export function createShortcutCaptureEvent(): string {
  return `shortcut-finished-${++captureEventId}`;
}

/** Preserve existing combinations, while allowing only Win as a standalone key. */
export function acceptedShortcut(keys: ShortcutCaptureResult): string[] | null {
  if (!keys?.length || keys.some((key) => !key.trim())) return null;

  const normalized = keys.map((key) => /^(?:[lr])?win$/i.test(key) ? "Win" : key);
  if (normalized.length === 1 && normalized[0] !== "Win") return null;
  return normalized;
}

/** Subscribe before requesting input: a fast completion must not lose its result. */
export function startShortcutCapture(
  ports: ShortcutCapturePorts,
  onAccepted: (keys: string[]) => void,
  onRejected: () => void = () => {},
): ShortcutCapture {
  let active = true;
  let unlisten: Unlisten | undefined;

  function release(listener: Unlisten) {
    try {
      Promise.resolve(listener()).catch((error) => console.error("Shortcut listener cleanup failed", error));
    } catch (error) {
      console.error("Shortcut listener cleanup failed", error);
    }
  }

  function cancel() {
    if (!active) return;
    active = false;
    if (unlisten) {
      release(unlisten);
      unlisten = undefined;
    }
  }

  const ready = (async () => {
    try {
      const listener = await ports.listen((keys) => {
        if (!active) return;
        cancel();
        const accepted = acceptedShortcut(keys);
        if (accepted) {
          onAccepted(accepted);
        } else if (keys?.length) {
          onRejected();
        }
        // Null is cancellation. An empty result retains the existing binding,
        // rather than accidentally clearing it or creating a global empty hotkey.
      });
      if (!active) {
        release(listener);
        return;
      }
      unlisten = listener;
      await ports.request();
    } catch (error) {
      cancel();
      throw error;
    }
  })();

  return { ready, cancel };
}
