import { invoke } from "@tauri-apps/api/core";
import { SeelenCommand } from "@seelen-ui/lib";
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
      return invoke(SeelenCommand.Lock);
    },
  },
  {
    key: "log_out",
    icon: "IoLogOutOutline",
    onClick() {
      return invoke(SeelenCommand.LogOut);
    },
  },
  {
    key: "shutdown",
    icon: "IoPower",
    onClick() {
      return invoke(SeelenCommand.Shutdown);
    },
  },
  {
    key: "reboot",
    icon: "MdRestartAlt",
    onClick() {
      return invoke(SeelenCommand.Restart);
    },
  },
  {
    key: "suspend",
    icon: "BiMoon",
    onClick() {
      return invoke(SeelenCommand.Suspend);
    },
  },
  {
    key: "hibernate",
    icon: "TbZzz",
    onClick() {
      return invoke(SeelenCommand.Hibernate);
    },
  },
];
