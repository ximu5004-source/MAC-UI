let dark = $state(document.documentElement.dataset.colorScheme === "dark");
class ColorScheme {
  get value() { return dark; }
}
export const prefersDarkColorScheme = new ColorScheme();

const update = () => { dark = document.documentElement.dataset.colorScheme === "dark"; };
const observer = new MutationObserver(update);
observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-color-scheme"] });
globalThis.addEventListener("macui-color-scheme-change", update);
