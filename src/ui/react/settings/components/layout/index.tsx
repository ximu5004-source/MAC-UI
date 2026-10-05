import { useLayoutEffect, useRef } from "preact/hooks";
import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Modal } from "antd";
import { useTranslation } from "react-i18next";
import { hasChanges, restoreToLastSaved } from "../../state/mod";
import { Outlet, useLocation } from "react-router";

import { Header } from "../header/index.tsx";
import { Navigation } from "../navigation/index.tsx";
import cs from "./index.module.css";

export function Layout() {
  const { t } = useTranslation();
  const location = useLocation();
  const contentRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    // Scroll to the top of the page when the route changes
    contentRef.current?.scrollTo({ top: 0, left: 0, behavior: "instant" });
  }, [location.pathname, location.search]);

  useEffect(() => {
    let disposed = false, prompting = false;
    let unlisten: (() => void) | undefined;
    const window = getCurrentWebviewWindow();
    void window.onCloseRequested((event) => {
      if (!hasChanges.value) return;
      event.preventDefault();
      if (prompting) return;
      prompting = true;
      Modal.confirm({
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
    <div className={cs.layout}>
      <Navigation />
      <Header />
      <main ref={contentRef} className={cs.content} id="settings-content">
        <Outlet />
      </main>
    </div>
  );
}
