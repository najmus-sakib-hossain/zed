import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: Workflows";
const description = "Compose repeatable engineering workflows with automation and shared context.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/workflows` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="Workflows"
      description="Design automation sequences and reusable templates for your team."
      bullets={[
        "Workflow graph fundamentals",
        "Trigger and action architecture",
        "Template versioning and sharing",
        "Cross-tool execution with MCP actions",
      ]}
    />
  );
}
