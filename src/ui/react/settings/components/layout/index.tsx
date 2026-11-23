import { useLayoutEffect, useRef } from "preact/hooks";
import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { settingsDialogs } from "../SettingsDialogs";
import { useTranslation } from "react-i18next";
import { hasChanges, restoreToLastSaved } from "../../state/mod";
import { Outlet, useLocation } from "react-router";

import { Header } from "../header/index.tsx";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";
import cs from "./index.module.css";

export function Layout() {
  const { t } = useTranslation();
  const location = useLocation();
  const contentRef = useRef<HTMLDivElement>(null);
  const params = new URLSearchParams(location.search);
  const find = params.get("find");
  const section = params.get("section");
  const resourceId = params.get("id");

  useEffect(() => {
    const frost = trackFrostedSurfaces(".mac-settings-surface");
    return () => frost.dispose();
  }, []);

  useLayoutEffect(() => {
    // Scroll to the top of the page when the route changes
    contentRef.current?.scrollTo({ top: 0, left: 0, behavior: "instant" });
    const target = Array.from(contentRef.current?.querySelectorAll<HTMLElement>("[data-section]") || [])
      .find((node) => node.dataset.section === section);
    const sectionTarget = target?.getClientRects().length
      ? target
      : target?.querySelector<HTMLElement>("[data-feature-header], h2, [data-setting]");
    sectionTarget?.scrollIntoView({ block: "start", behavior: "instant" });
    if (find) {
      const match = Array.from(contentRef.current?.querySelectorAll<HTMLElement>("[data-setting], summary") || [])
        .find((node) => node.textContent?.toLocaleLowerCase().includes(find.toLocaleLowerCase()));
      if (match) {
        let parent = match.parentElement;
        while (parent) {
          if (parent instanceof HTMLDetailsElement) parent.open = true;
          parent = parent.parentElement;
        }
        match.tabIndex = -1;
        match.focus({ preventScroll: true });
        match.scrollIntoView({ block: "center", behavior: "instant" });
      }
    }
  }, [location.pathname, find, section, resourceId]);

  useEffect(() => {
    let disposed = false, prompting = false;
    let unlisten: (() => void) | undefined;
    const window = getCurrentWebviewWindow();
    void window.onCloseRequested((event) => {
      if (!hasChanges.value) return;
      event.preventDefault();
      if (prompting) return;
      prompting = true;
      settingsDialogs.confirm({
        title: t("mac_ui.unsaved"),
        content: t("mac_ui.unsaved_hint"),
        centered: true,
        okText: t("mac_ui.discard"),
        cancelText: t("mac_ui.keep_editing"),
        okButtonProps: { danger: true },
        onOk: async () => {
          restoreToLastSaved();
          await window.destroy();
        },
        afterClose: () => {
          prompting = false;
        },
      });
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [t]);

  return (
    <div className={`${cs.layout} mac-settings-surface`}>
      <Header />
      <main ref={contentRef} className={cs.content} id="settings-content">
        <div key={location.pathname} className={cs.page}>
          <Outlet />
        </div>
      </main>
    </div>
  );
}
