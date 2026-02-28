import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: Getting Started";
const description = "Install DX, create your first project, and understand the connected runtime model.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/getting-started` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="Getting Started"
      description="Set up DX in minutes and launch your first connected workflow."
      bullets={[
        "Install DX desktop and CLI",
        "Create your first workspace",
        "Connect provider and local model",
        "Run your first assistant + tool workflow",
      ]}
    />
  );
}
