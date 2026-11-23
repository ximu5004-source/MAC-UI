import { useEffect, useState } from "react";

type Args = undefined | string | { [x: string]: any };
export const cx = (...args: Args[]): string => {
  return args
    .map((arg) => {
      if (!arg) {
        return;
      }

      if (typeof arg === "string") {
        return arg;
      }

      let classnames = "";
      Object.keys(arg).forEach((key) => {
        if (arg[key]) {
          classnames += ` ${key}`;
        }
      });

      return classnames.trimStart();
    })
    .join(" ");
};

export function isDarkModeEnabled() {
  return document.documentElement.dataset.colorScheme === "dark";
}

export function useDarkMode() {
  const [isDarkMode, setIsDarkMode] = useState(isDarkModeEnabled());

  useEffect(() => {
    const listener = () => setIsDarkMode(isDarkModeEnabled());
    const observer = new MutationObserver(listener);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-color-scheme"] });
    globalThis.addEventListener("macui-color-scheme-change", listener);
    listener();
    return () => {
      observer.disconnect();
      globalThis.removeEventListener("macui-color-scheme-change", listener);
    };
  }, []);

  return isDarkMode;
}
