import { settings } from "../../state/mod";
export function getStreamingMode(): boolean { return settings.value.streamingMode; }
export function setStreamingMode(streamingMode: boolean) {
  settings.value = { ...settings.value, streamingMode };
}
