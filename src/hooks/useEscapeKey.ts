import { useEffect } from "react";

/**
 * Calls `handler` when the Escape key is pressed, but only while `active` is true.
 */
export function useEscapeKey(handler: () => void, active: boolean) {
  useEffect(() => {
    if (!active) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handler();
      }
    };

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [handler, active]);
}
