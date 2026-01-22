import SwaggerUI from "swagger-ui-react";
import fullnodeSpec from "../../public/openapi-fullnode.json";
import walletSpec from "../../public/openapi-wallet.json";

interface SwaggerUIComponentProps {
  apiType: "fullnode" | "wallet";
}

export function SwaggerUIComponent({ apiType }: SwaggerUIComponentProps) {
  const spec = apiType === "fullnode" ? fullnodeSpec : walletSpec;

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
