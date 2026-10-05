# Launchpad motion regression — 2026-09-27

Run only the isolated frontend (no real applications, Windows preferences or user files):

```powershell
node --import tsx --test src/ui/svelte/apps-menu/launchpad.test.ts src/ui/svelte/apps-menu/pageMotion.test.ts src/ui/svelte/apps-menu/launchpadOperations.test.ts
node scripts/qa/launchpad-build.mjs
node scripts/qa/launchpad-server.mjs target/launchpad-motion-qa 3583
```

Open `http://127.0.0.1:3583/svelte/apps-menu/index.html?many-folder=1`.
Use `&motion=reduce` to test the reduced-motion JavaScript branch without changing OS settings.
The fixture reports animation and snapshot counts; app artwork is not supplied by this mock.

Verified in the in-app browser:

- Forward, backward, direct page jumps and fast alternating page clicks finish on the requested page.
- Animation peak: two (incoming/outgoing); snapshot peak: one; both return to zero when settled.
- Outgoing snapshots are inert and contain no `data-item-id` identities or focusable tabindex values.
- PageDown and horizontal scrolling both change pages in the expected direction.
- Searching during page changes removes old snapshots and resets to page one; Enter sends the correct mock launch request.
- A 24-application folder opens, changes between two pages and closes without changing the root page.
- Dragging one app onto another still creates a two-app folder and saves it in fixture memory.
- Reduced-motion branch: page changes work with animation/snapshot peaks both zero.
- Pure model tests: 16 passed, including placement serialization, rapid toggle parity, canceled opening, delayed hide ordering and external close.
- Svelte check: 2,003 files, zero errors, zero warnings at time of verification.

Not certified by this isolated fixture: real app launching, native wake/display recovery,
OS reduced-motion notification delivery and drag + page-button interaction on physical hardware.
The native display/visibility path needs the main integration test.
