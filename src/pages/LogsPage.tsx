import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal, ArrowDown } from "lucide-react";
import { useUIStore } from "@/store/useUIStore";
import type { LogSource, LogEntry } from "@/types";

function getLogLevelStyle(level: LogEntry["level"]) {
  switch (level) {
    case "error": return "text-rose-400";
    case "warning": return "text-[#9cf35b]";
    case "debug": return "text-white/30";
    default: return "text-white/60";
  }
}

function getSourceStyle(source: LogSource) {
  switch (source) {
    case "node": return "text-blue-400 bg-blue-400/10";
    case "miner": return "text-purple-400 bg-purple-400/10";
    case "headless": return "text-[#9cf35b] bg-[#9cf35b]/10";
  }
}

export function LogsPage() {
  const logs = useUIStore((s) => s.logs);
  const logFilters = useUIStore((s) => s.logFilters);
  const clearLogs = useUIStore((s) => s.clearLogs);
  const toggleLogFilter = useUIStore((s) => s.toggleLogFilter);
  const markLogsRead = useUIStore((s) => s.markLogsRead);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const isNearBottomRef = useRef(true);
  const [showScrollButton, setShowScrollButton] = useState(false);

  const filteredLogs = logs.filter((log) => logFilters.has(log.source));

  // Track whether the user is scrolled near the bottom
  const handleScroll = useCallback(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const threshold = 80; // px from bottom
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < threshold;
    isNearBottomRef.current = nearBottom;
    setShowScrollButton(!nearBottom);
  }, []);

  // Mark logs as read whenever this page is mounted or new logs arrive
  useEffect(() => {
    markLogsRead();
  }, [logs, markLogsRead]);

  // Only auto-scroll if user is near the bottom
  useEffect(() => {
    if (isNearBottomRef.current) {
      logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

  const scrollToBottom = useCallback(() => {
    isNearBottomRef.current = true;
    setShowScrollButton(false);
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  return (
    <>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-2xl font-bold text-white">Logs</h2>
          <p className="text-sm text-white/30 mt-1">Real-time service output</p>
        </div>
        <button
          onClick={clearLogs}
          className="px-4 py-2 rounded-lg bg-white/5 border border-white/5 text-white/70 text-sm font-medium hover:bg-white/5 transition-colors"
        >
          Clear Logs
        </button>
      </div>

      <div className="rounded-xl bg-[#0b0a12] border border-white/5 overflow-hidden flex-1 flex flex-col">
        <div className="flex items-center justify-between px-5 py-4 border-b border-white/5">
          <div className="flex items-center gap-3">
            <Terminal className="w-4 h-4 text-[#9cf35b]" />
            <h3 className="text-sm font-semibold text-white">Live Output</h3>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-white/5 text-white/30">
              {filteredLogs.length} / {logs.length}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-white/30 mr-2">Filter:</span>
            {(["node", "miner", "headless"] as LogSource[]).map((source) => (
              <button
                key={source}
                onClick={() => toggleLogFilter(source)}
                className={`px-3 py-1 rounded text-xs font-semibold uppercase transition-all ${
                  logFilters.has(source)
                    ? getSourceStyle(source)
                    : "text-white/25 bg-white/5 opacity-50"
                }`}
              >
                {source}
              </button>
            ))}
          </div>
        </div>
        <div className="relative flex-1 min-h-0">
          <div
            ref={scrollContainerRef}
            onScroll={handleScroll}
            className="absolute inset-0 overflow-auto bg-[#080b10] p-4 font-mono text-sm"
          >
            {filteredLogs.length > 0 ? (
              <div className="space-y-1">
                {filteredLogs.map((log) => (
                  <div key={log.id} className="flex gap-3 leading-relaxed hover:bg-white/3 px-2 py-1 rounded">
                    <span className="text-white/25 text-xs shrink-0">
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
              <div className="h-full flex items-center justify-center text-white/25">
                <div className="text-center">
                  <Terminal className="w-8 h-8 mx-auto mb-2 opacity-50" />
                  <p>{logs.length > 0 ? "No logs match the current filter." : "No logs yet. Start the network to see activity."}</p>
                </div>
              </div>
            )}
          </div>
          {showScrollButton && filteredLogs.length > 0 && (
            <button
              onClick={scrollToBottom}
              className="absolute bottom-4 right-6 p-2 rounded-full bg-white/10 border border-white/10 text-white/60 hover:bg-white/20 transition-colors shadow-lg"
              title="Scroll to bottom"
            >
              <ArrowDown className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>
    </>
  );
}
