import { useAppStore } from "@/store/useAppStore";

export function ErrorBanner() {
  const error = useAppStore((s) => s.error);

  if (!error) return null;

  return (
    <div className="bg-rose-500/10 border-b border-rose-500/30 px-6 py-3">
      <p className="text-sm text-rose-400 font-medium">{error}</p>
    </div>
  );
}
