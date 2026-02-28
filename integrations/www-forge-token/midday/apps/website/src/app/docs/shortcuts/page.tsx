import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxDocsTopic } from "@/components/dx-docs-topic";

const title = "DX Docs: Shortcuts";
const description = "Keyboard-first commands for navigation, generation, and execution in DX.";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical: `${baseUrl}/docs/shortcuts` },
};

export default function Page() {
  return (
    <DxDocsTopic
      title="Shortcuts"
      description="Master keyboard-driven workflows and cut context-switch time."
      bullets={[
        "Global command palette patterns",
        "Editor, terminal, and assistant shortcut sets",
        "Custom keymap and shortcut conflict handling",
        "Workflow trigger shortcuts",
      ]}
    />
  );
}
