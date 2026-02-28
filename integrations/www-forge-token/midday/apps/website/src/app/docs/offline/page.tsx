import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: Offline";
const description = "Operate DX fully offline with local models, cached context, and resilient sync.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/offline` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="Offline"
      description="Use DX without internet and maintain productivity across disconnected sessions."
      bullets={[
        "Configure local model runtime",
        "Cache docs and workspace context",
        "Deferred sync and conflict handling",
        "Offline security and local encryption",
      ]}
    />
  );
}
