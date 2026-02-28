import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: MCP Apps";
const description = "Install and build MCP apps to extend DX assistant capabilities across your stack.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/mcp-apps` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="MCP Apps"
      description="Connect MCP-compatible apps and give your assistant direct tool access."
      bullets={[
        "Browse and install from MCP app catalog",
        "Permission model and security boundaries",
        "Step-by-step MCP integration lifecycle",
        "Build and publish custom MCP apps",
      ]}
    />
  );
}
