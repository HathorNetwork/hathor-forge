import { useEffect, useRef } from "react";
import { Terminal } from "lucide-react";
import { useUIStore } from "@/store/useUIStore";
import type { LogSource, LogEntry } from "@/types";

function getLogLevelStyle(level: LogEntry["level"]) {
  switch (level) {
    case "error": return "text-rose-400";
    case "warning": return "text-amber-400";
    case "debug": return "text-slate-500";
    default: return "text-emerald-400";
  }
}

function getSourceStyle(source: LogSource) {
  switch (source) {
    case "node": return "text-blue-400 bg-blue-400/10";
    case "miner": return "text-purple-400 bg-purple-400/10";
    case "headless": return "text-amber-400 bg-amber-400/10";
  }
}

export function LogsPage() {
  const logs = useUIStore((s) => s.logs);
  const logFilters = useUIStore((s) => s.logFilters);
  const clearLogs = useUIStore((s) => s.clearLogs);
  const toggleLogFilter = useUIStore((s) => s.toggleLogFilter);
  const markLogsRead = useUIStore((s) => s.markLogsRead);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const filteredLogs = logs.filter((log) => logFilters.has(log.source));

  // Mark logs as read whenever this page is mounted or new logs arrive
  useEffect(() => {
    markLogsRead();
  }, [logs, markLogsRead]);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-white">Logs</h2>
          <p className="text-sm text-slate-500 mt-1">Real-time service output</p>
        </div>
        <button
          onClick={clearLogs}
          className="px-4 py-2 rounded-lg bg-slate-800 border border-slate-700 text-slate-300 text-sm font-medium hover:bg-slate-700 transition-colors"
        >
          Clear Logs
        </button>
      </div>

      <div className="rounded-xl bg-[#0d1117] border border-slate-800/50 overflow-hidden flex-1 flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800/50">
          <div className="flex items-center gap-3">
            <Terminal className="w-4 h-4 text-amber-400" />
            <h3 className="text-sm font-semibold text-white">Live Output</h3>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-slate-800 text-slate-400">
              {filteredLogs.length} / {logs.length}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-slate-500 mr-2">Filter:</span>
            {(["node", "miner", "headless"] as LogSource[]).map((source) => (
              <button
                key={source}
                onClick={() => toggleLogFilter(source)}
                className={`px-3 py-1 rounded text-xs font-semibold uppercase transition-all ${
                  logFilters.has(source)
                    ? getSourceStyle(source)
                    : "text-slate-600 bg-slate-800/50 opacity-50"
                }`}
              >
                {source}
              </button>
            ))}
          </div>
        </div>
        <div className="flex-1 overflow-auto bg-[#080b10] p-4 font-mono text-sm min-h-0" style={{ maxHeight: 'calc(100vh - 280px)' }}>
          {filteredLogs.length > 0 ? (
            <div className="space-y-1">
              {filteredLogs.map((log) => (
                <div key={log.id} className="flex gap-3 leading-relaxed hover:bg-slate-900/30 px-2 py-1 rounded">
                  <span className="text-slate-600 text-xs shrink-0">
                    {log.timestamp.toLocaleTimeString()}
                  </span>
                  <span
                    className={`text-xs font-semibold uppercase shrink-0 w-16 px-1.5 py-0.5 rounded ${getSourceStyle(log.source)}`}
                  >
                    {log.source}
                  </span>
                  <span className={`${getLogLevelStyle(log.level)} break-all`}>{log.message}</span>
                </div>
              ))}
              <div ref={logsEndRef} />
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-slate-600">
              <div className="text-center">
                <Terminal className="w-8 h-8 mx-auto mb-2 opacity-50" />
                <p>{logs.length > 0 ? "No logs match the current filter." : "No logs yet. Start the network to see activity."}</p>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );
}
