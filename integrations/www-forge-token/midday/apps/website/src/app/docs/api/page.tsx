import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: API";
const description = "API and SDK references for integrating DX into custom engineering systems.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/api` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="API"
      description="Integrate DX capabilities into your own platform via API, SDK, and event hooks."
      bullets={[
        "Authentication and project scopes",
        "Assistant and workflow endpoints",
        "MCP app lifecycle API",
        "Webhooks and event subscriptions",
      ]}
    />
  );
}
