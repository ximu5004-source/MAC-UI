import { Icon } from "libs/ui/react/components/Icon";
import { Input, Modal } from "antd";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router";
import { searchSettings, sections } from "../../modules/Hub/catalog";
import cs from "./index.module.css";

export function SettingsSearch() {
  const { t, i18n } = useTranslation();
  const location = useLocation();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const input = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  function close() {
    setOpen(false);
    setQuery("");
  }
  useEffect(() => {
    function key(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setOpen(true);
      }
    }
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, []);
  useEffect(close, [location.pathname, location.search]);
  const items = query.trim()
    ? searchSettings(query, t, i18n)
    : sections.map((item) => ({ ...item, label: t(`settings_home.${item.key}`), hint: undefined }));
  return (
    <>
      <button
        type="button"
        ref={trigger}
        className={cs.searchButton}
        aria-label={t("mac_ui.search")}
        title={t("mac_ui.search") + " (Ctrl+K)"}
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
      >
        <Icon iconName="TbSearch" size={18} />
        <span>{t("mac_ui.search")}</span>
        <kbd>Ctrl K</kbd>
      </button>
      <Modal
        title={t("mac_ui.search")}
        open={open}
        footer={null}
        onCancel={close}
        width={540}
        centered
        afterOpenChange={(visible) =>
          visible ? input.current?.querySelector("input")?.focus() : trigger.current?.focus()}
      >
        <div ref={input}>
          <Input
            allowClear
            aria-label={t("mac_ui.search")}
            placeholder={t("mac_ui.search")}
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                document.getElementById("settings-search-results")?.querySelector("a")?.focus();
              }
            }}
          />
        </div>
        <div
          id="settings-search-results"
          className={cs.results}
          onKeyDown={(event) => {
            const links = Array.from(event.currentTarget.querySelectorAll("a"));
            const index = links.indexOf(document.activeElement as HTMLAnchorElement);
            if (event.key === "ArrowDown") {
              event.preventDefault();
              links[Math.min(index + 1, links.length - 1)]?.focus();
            }
            if (event.key === "ArrowUp") {
              event.preventDefault();
              index > 0 ? links[index - 1]?.focus() : input.current?.querySelector("input")?.focus();
            }
          }}
        >
          {items.map((item) => (
            <NavLink key={item.route + item.label} to={item.route} onClick={close}>
              <Icon iconName={item.icon as any} />
              <span>{item.label}{item.hint && <small>{item.hint}</small>}</span>
              <Icon iconName="TbChevronRight" />
            </NavLink>
          ))}
          {!items.length && <p role="status">{t("mac_ui.no_results")}</p>}
        </div>
      </Modal>
    </>
  );
}
