import { X } from "lucide-react";
import { useUIStore } from "@/store/useUIStore";

export function ErrorBanner() {
  const error = useUIStore((s) => s.error);
  const setError = useUIStore((s) => s.setError);

  if (!error) return null;

  return (
    <div className="bg-rose-500/10 border-b border-rose-500/30 px-6 py-3 flex items-center justify-between">
      <p className="text-sm text-rose-400 font-medium">{error}</p>
      <button
        onClick={() => setError(null)}
        className="p-1 rounded hover:bg-rose-500/20 transition-colors text-rose-400 hover:text-rose-300"
        aria-label="Dismiss error"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}
