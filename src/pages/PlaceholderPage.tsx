export function PlaceholderPage({ title }: { title: string }) {
  return (
    <div className="flex items-center justify-center h-full">
      <div className="text-center">
        <h2 className="text-2xl font-bold text-white mb-2">{title}</h2>
        <p className="text-slate-500">Coming soon</p>
      </div>
    </div>
  );
}
