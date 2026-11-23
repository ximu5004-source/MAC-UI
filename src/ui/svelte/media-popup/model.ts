import type { MediaPlayer } from "@macui/lib/types";

/** Prefer the OS-selected player; if unavailable, use an active player, then the first session. */
export function selectNowPlayingSession(sessions: readonly MediaPlayer[]): MediaPlayer | undefined {
  return sessions.find((session) => session.default) ?? sessions.find((session) => session.playing) ?? sessions[0];
}

export type MediaPopupView = "main" | "mixer" | "now-playing";

/** Keep the existing volume entry point as the default for absent or untrusted custom arguments. */
export function mediaPopupViewFromArgs(args: Record<string, unknown> | null | undefined): MediaPopupView {
  return args?.view === "now-playing" ? "now-playing" : "main";
}
