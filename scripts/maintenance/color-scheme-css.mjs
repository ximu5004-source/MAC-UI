import fs from 'node:fs';
const file = 'libs/core/styles/colors.css';
let source = fs.readFileSync(file, 'utf8');
source = source.replace('color-scheme: light dark;', 'color-scheme: light;').replace(
  /@media \(prefers-color-scheme: (dark|light)\) \{\s*:root \{([\s\S]*?)\n  \}\n\}/g,
  (_, mode, body) => `:root${mode === 'dark' ? '[data-color-scheme="dark"]' : ':not([data-color-scheme="dark"])'} {${body}\n}`,
);
fs.writeFileSync(file, source);
for (const file of ['libs/core/styles/reset.css', 'src/ui/vanilla/third_party/reset.css']) {
  fs.writeFileSync(file, fs.readFileSync(file, 'utf8').replaceAll('color-scheme: light dark;', 'color-scheme: light;'));
}
for (const file of ['src/ui/svelte/apps-menu/launchpad.css', 'src/ui/svelte/calendar-popup/calendar.css', 'src/ui/svelte/power-menu/power-menu.css']) {
  fs.writeFileSync(file, fs.readFileSync(file, 'utf8').replaceAll('color-scheme: light dark;', 'color-scheme: inherit;'));
}
