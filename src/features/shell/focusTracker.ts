// Which app window has focus, shared between windows through localStorage (every window has the same origin), so
// the owning window knows whether any window is focused before it sends an OS notification.
import type { WindowId } from "@/lib/bindings";

const KEY = "tc:focused-window";

export function focusedWindow(): WindowId | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

/** Records this window's focus while mounted; returns the uninstall. */
export function trackWindowFocus(windowId: WindowId): () => void {
  const onFocus = () => {
    try {
      localStorage.setItem(KEY, windowId);
    } catch {
      // Storage blocked: the owner then assumes no window is focused.
    }
  };
  const onBlur = () => {
    try {
      if (localStorage.getItem(KEY) === windowId) localStorage.removeItem(KEY);
    } catch {
      // As above.
    }
  };
  if (document.hasFocus()) onFocus();
  window.addEventListener("focus", onFocus);
  window.addEventListener("blur", onBlur);
  return () => {
    onBlur();
    window.removeEventListener("focus", onFocus);
    window.removeEventListener("blur", onBlur);
  };
}
