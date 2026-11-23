import { invoke, MacUiCommand } from "@macui/lib";
import type {
  ResourceId,
  ResourceKind,
  ResourceMetadata,
  ResourceText as IResourceText,
  Wallpaper,
} from "@macui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { ResourceText, ResourceTextAsMarkdown } from "libs/ui/react/components/ResourceText/index.tsx";
import { cx } from "libs/ui/react/utils/styling.ts";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Button, Flex, Input, Popconfirm, Tooltip } from "antd";
import type { ComponentChildren } from "preact";
import { useTranslation } from "react-i18next";

import { SettingsGroup } from "../../components/SettingsBox/index.tsx";
import { EnvConfig } from "../shared/config/infra.ts";
import cs from "./infra.module.css";
import type { IconName } from "libs/ui/icons.ts";
import { $corruptedWallpapers } from "../shared/signals.ts";

type AnyResource = {
  id: ResourceId;
  metadata: ResourceMetadata;
  icon?: string | null;
};

interface ResourceCardProps {
  kind: ResourceKind;
  resource: AnyResource;
  actions?: React.ReactNode;
  body?: React.ReactNode;
}

export function ResourceCard({ resource, kind, actions, body }: ResourceCardProps) {
  const isCorrupted = kind === "Wallpaper" && $corruptedWallpapers.value.has(resource.id);

  const { t } = useTranslation();

  const [major = 0, minor = 0, patch = 0] = EnvConfig.version.split(".").map(Number);
  const [majorTarget = 0, minorTarget = 0, patchTarget = 0] = resource.metadata.appTargetVersion || [];

  const targetIsOlder = !!resource.metadata.appTargetVersion &&
    (majorTarget < major ||
      (majorTarget === major && minorTarget < minor) ||
      (majorTarget === major && minorTarget === minor && patchTarget < patch));

  const targetIsNewer = !!resource.metadata.appTargetVersion &&
    (majorTarget > major ||
      (majorTarget === major && minorTarget > minor) ||
      (majorTarget === major && minorTarget === minor && patchTarget > patch));

  return (
    <div
      className={cx(cs.card, {
        [cs.warn!]: targetIsOlder,
        [cs.danger!]: targetIsNewer || isCorrupted,
      })}
    >
      <ResourcePortrait resource={resource} kind={kind}>
        {targetIsOlder && (
          <Tooltip title={t("resources.outdated")}>
            <Icon iconName="IoWarning" className={cs.warning} />
          </Tooltip>
        )}
        {targetIsNewer && (
          <Tooltip title={t("resources.app_outdated")}>
            <Icon iconName="IoWarning" className={cs.danger} />
          </Tooltip>
        )}
        {isCorrupted && (
          <Tooltip title={t("resources.corrupted_wallpaper")}>
            <Icon iconName="MdErrorOutline" className={cs.corrupted} />
          </Tooltip>
        )}
      </ResourcePortrait>

      <div className={cs.header}>
        <ResourceText className={cs.title} text={resource.metadata.displayName} />

        <div className={cs.actionsTop}>
          {actions}
        </div>
      </div>

      <div className={cs.body}>
        {body || <ResourceTextAsMarkdown text={resource.metadata.description} />}
      </div>

      <div className={cs.footer}>
        {!resource.metadata.bundled &&
          ["com.macui.macui", "com.jona.mac-ui"].some((id) => resource.metadata.path.includes(id)) && (
          <Tooltip title={t("resources.delete")} placement="left">
            <Popconfirm
              title={t("action.confirm")}
              description={t("action.confirm_body")}
              okText={t("yes")}
              cancelText={t("no")}
              onConfirm={() => {
                invoke(MacUiCommand.RemoveResource, { kind, id: resource.id });
              }}
            >
              <Button type="text" danger>
                <Icon iconName="BiTrash" />
              </Button>
            </Popconfirm>
          </Tooltip>
        )}
      </div>
    </div>
  );
}

const icons: Record<ResourceKind, IconName> = {
  Theme: "IoColorPaletteOutline",
  IconPack: "LiaIconsSolid",
  Plugin: "BsPlugin",
  Widget: "BiSolidWidget",
  Wallpaper: "LuWallpaper",
  SoundPack: "PiWaveformBold",
};

interface ResourcePortraitProps {
  resource: AnyResource;
  kind: ResourceKind;
  children?: ComponentChildren;
}

export function ResourceIcon({ kind }: { kind: ResourceKind }) {
  return <Icon className={cs.kindIcon} iconName={icons[kind]!} />;
}

function ResourcePortraitInner({ resource, kind }: ResourcePortraitProps) {
  if (resource.metadata.portrait && !/^https?:/i.test(resource.metadata.portrait)) {
    return <img src={resource.metadata.portrait} />;
  }

  if (kind === "Wallpaper") {
    const wallpaper = resource as Wallpaper;
    if (wallpaper.thumbnailFilename) {
      return (
        <img
          src={convertFileSrc(`${resource.metadata.path}\\${wallpaper.thumbnailFilename}`)}
          style={{ filter: "blur(0.4px)" }}
          loading="lazy"
        />
      );
    }
  }

  if (resource.icon) {
    return <Icon className={cs.kindIcon} iconName={resource.icon as IconName} />;
  }

  return <ResourceIcon kind={kind} />;
}

export function ResourcePortrait({ resource, kind, children }: ResourcePortraitProps) {
  return (
    <figure className={cs.portrait}>
      <ResourcePortraitInner resource={resource} kind={kind} />
      {children}
    </figure>
  );
}

export function ResourceDescription({ text }: { text?: IResourceText }) {
  if (!text) return null;
  return (
    <div className={cs.description}>
      <ResourceTextAsMarkdown text={text} />
    </div>
  );
}

export function resolveDisplayName(name: IResourceText | undefined, language: string): string {
  if (!name) return "";
  if (typeof name === "string") return name;
  return name[language] ?? name["en"] ?? "";
}

interface ResourceListHeaderProps {
  search: string;
  onSearch: (v: string) => void;
  children?: ComponentChildren;
}

export function ResourceListHeader({
  search,
  onSearch,
  children,
}: ResourceListHeaderProps) {
  const { t } = useTranslation();
  return (
    <SettingsGroup>
      <Flex gap={10} align="center">
        <b style={{ flexShrink: 0 }}>{t("search")}:</b>
        <Input
          type="search"
          value={search}
          onChange={(e) => onSearch((e.target as HTMLInputElement).value)}
          style={{ flex: 1, minWidth: 0 }}
        />
      </Flex>

      <p style={{ color: "var(--mac-secondary)", fontSize: 13 }}>{t("mac_local.wallpapers_local_hint")}</p>
      {children}
    </SettingsGroup>
  );
}
