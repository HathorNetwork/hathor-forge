import { useEffect } from "react";
import { useUIStore } from "./store/useUIStore";
import { useNodeStatusPolling } from "@/hooks/useNodeStatus";
import { useContractStatesPolling } from "@/hooks/useContractStates";
import { useTauriEvents } from "@/hooks/useTauriEvents";
import { useInitialStateReconciliation } from "@/hooks/useInitialStateReconciliation";
import { usePortsStore } from "@/store/usePortsStore";
import { getPorts } from "@/services/tauri";

import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Sidebar } from "@/components/layout/Sidebar";
import { Header } from "@/components/layout/Header";
import { ErrorBanner } from "@/components/layout/ErrorBanner";

import { DashboardPage } from "@/pages/DashboardPage";
import { ExplorerPage } from "@/pages/ExplorerPage";
import { WalletPage } from "@/pages/WalletPage";
import { NanoContractsPage } from "@/pages/NanoContractsPage";
import { LogsPage } from "@/pages/LogsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { ApiExplorerPage } from "@/pages/ApiExplorerPage";


function App() {
  const currentPage = useUIStore((s) => s.currentPage);
  const setPorts = usePortsStore((s) => s.setPorts);

  // Fetch the dynamically-assigned ports from the Rust backend once on startup.
  useEffect(() => {
    getPorts().then(setPorts).catch(() => {/* keep defaults */});
  }, [setPorts]);

  // Global hooks: polling, event subscriptions & initial state
  useTauriEvents();
  useInitialStateReconciliation();
  useNodeStatusPolling();
  useContractStatesPolling();

  const renderContent = () => {
    switch (currentPage) {
      case "dashboard": return <DashboardPage />;
      case "explorer": return <ExplorerPage />;
      case "wallet": return <WalletPage />;
      case "nano-contracts": return <NanoContractsPage />;
      case "logs": return <LogsPage />;
      case "api-explorer": return <ApiExplorerPage />;
      case "settings": return <SettingsPage />;
      default: return <DashboardPage />;
    }
  };

  return (
    <div className="flex h-screen bg-[#0a0e14] text-slate-200 font-sans relative">
      {/* Full-width drag region at top for window dragging */}
      <div className="absolute top-0 left-0 right-0 h-8 select-none z-50" data-tauri-drag-region />
      <Sidebar />
      <main className="flex-1 flex flex-col overflow-hidden" role="main" aria-label="Main content">
        <Header />
        <ErrorBanner />
        <div className="flex-1 overflow-auto p-6">
          <ErrorBoundary>
            {renderContent()}
          </ErrorBoundary>
        </div>
      </main>
    </div>
  );
}

export default App;
