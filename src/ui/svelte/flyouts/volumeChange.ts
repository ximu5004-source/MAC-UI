interface VolumeState {
  volume: number;
  muted: boolean;
}

/** Display volume changes at the same integer-percent precision as the OSD. */
export function didVolumeChange(previous: VolumeState, current: VolumeState | null | undefined): boolean {
  if (!current) return false;
  return previous.volume.toFixed(2) !== current.volume.toFixed(2) || previous.muted !== current.muted;
}
