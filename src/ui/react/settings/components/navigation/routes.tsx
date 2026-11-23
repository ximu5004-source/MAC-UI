import { Icon } from "libs/ui/react/components/Icon";
import type React from "react";

export enum RoutePath {
  Home = "/",
  General = "/general",
  Wallpapers = "/wallpapers",
  Shortcuts = "/shortcuts",
  SettingsByMonitor = "/monitors",
  SettingsByApplication = "/specific_apps",
  DevTools = "/developer",
  Extras = "/extras",
}

export const RouteIcons: { [key in RoutePath]?: React.ReactNode } = {
  [RoutePath.Home]: <Icon iconName="TbHome" />,
  [RoutePath.General]: <Icon iconName="RiSettings3Fill" />,
  [RoutePath.Wallpapers]: <Icon iconName="TbWallpaper" />,
  [RoutePath.SettingsByMonitor]: <Icon iconName="PiMonitorBold" />,
  [RoutePath.SettingsByApplication]: <Icon iconName="IoIosApps" />,
  [RoutePath.Shortcuts]: <Icon iconName="MdLaunch" />,
  [RoutePath.Extras]: <Icon iconName="PiInfoFill" />,
  [RoutePath.DevTools]: <Icon iconName="PiCodeBold" />,
};
