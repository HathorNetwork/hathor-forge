import { useAppStore } from "./store/useAppStore";
import { useNodeStatusPolling } from "@/hooks/useNodeStatus";
import { useContractStatesPolling } from "@/hooks/useContractStates";
import { useTauriEvents } from "@/hooks/useTauriEvents";
import { useInitialStateReconciliation } from "@/hooks/useInitialStateReconciliation";

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
  const currentPage = useAppStore((s) => s.currentPage);

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
    <div className="flex h-screen bg-[#0a0e14] text-slate-200 font-sans">
      <Sidebar />
      <main className="flex-1 flex flex-col overflow-hidden">
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
