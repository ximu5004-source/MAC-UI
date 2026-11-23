import { window as TauriWindow } from "@macui/lib/tauri";
import { minimizeTarget } from "./minimizeTarget";

/** Both minimize and restore must use the actual physical Dock icon bounds. */
export async function captureDockTarget(item: HTMLElement) {
  // Only measure here; the backend applies the explicit window-animation
  // preference independently from Windows' native animation setting.
  let timeout: ReturnType<typeof setTimeout> | undefined;
  try {
    const origin = await Promise.race([
      TauriWindow.getCurrentWindow().innerPosition(),
      new Promise<never>((_, reject) => {
        timeout = setTimeout(() => reject(new Error("Dock geometry timeout")), 60);
      }),
    ]);
    const icon = item.querySelector(".weg-item-icon") ?? item;
    return minimizeTarget(icon.getBoundingClientRect(), origin, globalThis.devicePixelRatio);
  } catch { return null; }
  finally { clearTimeout(timeout); }
}
