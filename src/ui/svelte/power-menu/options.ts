import { invoke } from "@tauri-apps/api/core";
import { MacUiCommand } from "@macui/lib";
import type { PowerAction } from "./actionPolicy";

export interface Option {
  key: PowerAction;
  icon: string;
  onClick: () => Promise<void>;
}

export const options: Option[] = [
  {
    key: "lock",
    icon: "IoLockClosed",
    onClick() {
      return invoke(MacUiCommand.Lock);
    },
  },
  {
    key: "log_out",
    icon: "IoLogOutOutline",
    onClick() {
      return invoke(MacUiCommand.LogOut);
    },
  },
  {
    key: "shutdown",
    icon: "IoPower",
    onClick() {
      return invoke(MacUiCommand.Shutdown);
    },
  },
  {
    key: "reboot",
    icon: "MdRestartAlt",
    onClick() {
      return invoke(MacUiCommand.Restart);
    },
  },
  {
    key: "suspend",
    icon: "BiMoon",
    onClick() {
      return invoke(MacUiCommand.Suspend);
    },
  },
  {
    key: "hibernate",
    icon: "TbZzz",
    onClick() {
      return invoke(MacUiCommand.Hibernate);
    },
  },
];
