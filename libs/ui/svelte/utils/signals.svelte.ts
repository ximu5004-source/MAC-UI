import { MacUiEvent, subscribe, Widget } from "@macui/lib";

let _isThisWebviewFocused = $state(false);
subscribe(MacUiEvent.GlobalFocusChanged, ({ payload: { hwnd, ownerHwnd } }) => {
  _isThisWebviewFocused = Widget.self.windowId === hwnd || Widget.self.windowId === ownerHwnd;
});

export const isThisWebviewFocused = {
  get value() {
    return _isThisWebviewFocused;
  },
};

const _pointerQuery = window.matchMedia("(hover: hover) and (pointer: fine)");
let _isTouchPrimary = $state(!_pointerQuery.matches);
_pointerQuery.addEventListener("change", (e) => {
  _isTouchPrimary = !e.matches;
});

export const isTouchPrimary = {
  get value() {
    return _isTouchPrimary;
  },
};
