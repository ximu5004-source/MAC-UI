export type PowerAction = "lock" | "log_out" | "shutdown" | "reboot" | "suspend" | "hibernate";
export function needsPowerConfirmation(action: PowerAction): boolean {
  return action === "shutdown" || action === "reboot" || action === "log_out";
}
