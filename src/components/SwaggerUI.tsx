import { useEffect, useState } from "react";
import SwaggerUI from "swagger-ui-react";

interface SwaggerUIComponentProps {
  apiType: "fullnode" | "wallet";
}

export function SwaggerUIComponent({ apiType }: SwaggerUIComponentProps) {
  const [spec, setSpec] = useState<object | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    setSpec(null);

    const specPath =
      apiType === "fullnode"
        ? "/openapi-fullnode.json"
        : "/openapi-wallet.json";

    fetch(specPath)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`Failed to fetch spec: ${response.statusText}`);
        }
        return response.json();
      })
      .then((data) => {
        setSpec(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err.message || "Failed to load API specification");
        setLoading(false);
      });
  }, [apiType]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-slate-400">Loading API specification...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-red-400">Error: {error}</div>
      </div>
    );
  }

  if (!spec) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-slate-400">No specification loaded</div>
      </div>
    );
  }

  return (
    <div className="swagger-ui-wrapper">
      <SwaggerUI
        spec={spec}
        docExpansion="list"
        filter={true}
        tryItOutEnabled={true}
        persistAuthorization={true}
        displayRequestDuration={true}
      />
    </div>
  );
}
