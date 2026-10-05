import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { debounce } from "lodash";
import z from "zod";
import { focusedWinId, monitors, previews, settings, widget, windows } from "./getters.svelte.ts";

export { focusedWinId, monitors, previews, settings, widget, windows };

const WidgetConfigSchema = z.object({
  onlyOnActiveMonitor: z.boolean(),
});

const widgetConfig = $derived.by(
  () =>
    WidgetConfigSchema.safeParse(settings.value.getCurrentWidgetConfig()).data ??
      (widget.getDefaultConfig() as unknown as z.infer<typeof WidgetConfigSchema>),
);

// +++++++++++++++++++++++ Reactive State +++++++++++++++++++++++

let showing = $state(false);
let autoConfirm = $state(false);

let desiredPosition = $state<{ x: number; y: number } | null>(null);

let selectedWindow = $state<number | null>(focusedWinId.value ?? null);

// Sync selectedWindow with focused window when the switcher is not visible
$effect.root(() => {
  $effect(() => {
    if (!showing) {
      const win = filteredWindows.find((w) => w.hwnd === focusedWinId.value);
      selectedWindow = win?.hwnd ?? null;
    }
  });
});

let desktopRect = $derived.by(() => {
  let rect = { top: 0, left: 0, right: 0, bottom: 0 };
  for (const monitor of monitors.value) {
    rect.left = Math.min(rect.left, monitor.rect.left);
    rect.top = Math.min(rect.top, monitor.rect.top);
    rect.right = Math.max(rect.right, monitor.rect.right);
    rect.bottom = Math.max(rect.bottom, monitor.rect.bottom);
  }
  return rect;
});

// Monitor under the cursor position that triggered the switcher, falling back to primary
let activeMonitor = $derived.by(() => {
  const pos = desiredPosition;
  const found = pos &&
    monitors.value.find(
      (m) =>
        m.rect.left <= pos.x &&
        pos.x < m.rect.right &&
        m.rect.top <= pos.y &&
        pos.y < m.rect.bottom,
    );
  return found || monitors.value.find((m) => m.isPrimary) || monitors.value[0];
});

// Windows shown in the switcher, optionally restricted to the active monitor
let filteredWindows = $derived.by(() => {
  const monitor = activeMonitor;
  if (!widgetConfig.onlyOnActiveMonitor || !monitor) {
    return windows.value;
  }
  return windows.value.filter((w) => w.monitor === monitor.id);
});

let relativeActiveMonitor = $derived.by(() => {
  const monitor = activeMonitor;
  if (!monitor) {
    return null;
  }
  return {
    ...monitor,
    rect: {
      top: monitor.rect.top - desktopRect.top,
      left: monitor.rect.left - desktopRect.left,
      right: monitor.rect.right - desktopRect.left,
      bottom: monitor.rect.bottom - desktopRect.top,
    },
  };
});

$effect.root(() => {
  widget.attachPosition();
  $effect(() => {
    widget.setPosition(desktopRect);
  });
});

// +++++++++++++++++++++++ State Class +++++++++++++++++++++++

class State {
  get showing() {
    return showing;
  }

  set showing(value: boolean) {
    showing = value;
  }

  get windows() {
    return filteredWindows;
  }

  get previews() {
    return previews.value;
  }

  get selectedWindow() {
    return selectedWindow;
  }

  set selectedWindow(value: number | null) {
    selectedWindow = value;
  }

  get activeMonitor() {
    return relativeActiveMonitor;
  }
}

export const globalState = new State();

// +++++++++++++++++++++++ Visibility +++++++++++++++++++++++

$effect.root(() => {
  $effect(() => {
    let cancelled = false;

    if (showing) {
      widget.show().then(async () => {
        if (!cancelled) {
          await widget.focus();
        }
      });
    } else {
      widget.hide();
    }

    return () => {
      cancelled = true;
    };
  });

  const hideIfNotFocused = debounce(() => {
    if (focusedWinId.value !== widget.windowId) {
      showing = false;
    }
  }, 100);
  // Hide when focus leaves the widget
  $effect(() => {
    focusedWinId.value; // subscribed
    hideIfNotFocused(); // debounced to avoid inmediate hidden if focused changed while opening the widget
  });

  // Poll the hardware Alt key state instead of relying on keyup events,
  // since the widget-focus trick fakes an Alt keydown that never reaches window.onkeyup.
  $effect(() => {
    if (!showing) {
      return;
    }

    let cancelled = false;
    let wasAltDown = true;

    const poll = async () => {
      while (!cancelled) {
        const isAltDown = await invoke(SeelenCommand.GetKeyState, { key: "Alt" });
        if (wasAltDown && !isAltDown) {
          onAltKeyUp();
        }
        wasAltDown = isAltDown;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
    };
    poll();

    return () => {
      cancelled = true;
    };
  });
});

// +++++++++++++++++++++++ Triggering +++++++++++++++++++++++

function onAltKeyUp() {
  if (showing && selectedWindow && autoConfirm) {
    showing = false;
    invoke(SeelenCommand.WegToggleWindowState, {
      hwnd: selectedWindow,
      wasFocused: false,
      minimizeTarget: null,
    });
  }
}

widget.onTrigger((payload) => {
  const direction: string = (payload.customArgs?.direction as string) || "next";
  const autoConfirmValue: boolean = (payload.customArgs?.autoConfirm as boolean) || false;

  // Only capture autoConfirm and the trigger monitor on the first trigger (when switcher was hidden),
  // and do it before filtering windows so the monitor filter reflects the new cursor position.
  if (!showing) {
    autoConfirm = autoConfirmValue;
    if (payload.desiredPosition) {
      desiredPosition = payload.desiredPosition;
    }
  }

  const targetWindows = filteredWindows;
  if (targetWindows.length === 0) {
    return;
  }

  // Use the currently selected window when already showing, otherwise start from focused
  const currentHwnd = showing ? selectedWindow : focusedWinId.value;

  let index = targetWindows.findIndex((w) => w.hwnd === currentHwnd);
  if (direction === "next") {
    if (index === -1) index = targetWindows.length - 1;
    selectedWindow = targetWindows[(index + 1) % targetWindows.length]?.hwnd ?? null;
  } else if (direction === "previous") {
    if (index === -1) index = 0;
    selectedWindow = targetWindows[(index - 1 + targetWindows.length) % targetWindows.length]?.hwnd ?? null;
  }

  showing = true;
});

window.onkeydown = (e) => {
  if (e.key === "Escape") {
    showing = false;
  }
};
