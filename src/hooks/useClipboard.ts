import { useState, useCallback } from "react";

export function useClipboard(resetDelay = 2000) {
  const [copiedValue, setCopiedValue] = useState<string | null>(null);

  const copy = useCallback(
    async (text: string, key?: string) => {
      await navigator.clipboard.writeText(text);
      setCopiedValue(key ?? text);
      setTimeout(() => setCopiedValue(null), resetDelay);
    },
    [resetDelay]
  );

  return { copiedValue, copy };
}
