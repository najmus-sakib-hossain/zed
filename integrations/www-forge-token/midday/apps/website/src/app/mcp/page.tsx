import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { MCP } from "@/components/mcp";

const title = "DX MCP — Model Context Protocol Integrations";
const description =
  "Connect AI tools to your development environment via MCP. Give Cursor, Claude, ChatGPT, Raycast, Copilot, and Zapier live access to your files, APIs, CLIs, and workflow tools through DX.";

export const metadata: Metadata = {
  title,
  description,
  keywords: [
    "MCP",
    "Model Context Protocol",
    "AI integration",
    "Claude MCP",
    "Cursor MCP",
    "developer tool automation",
    "DX MCP apps",
    "AI workflow integration",
  ],
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/mcp`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/mcp`,
  },
};

export default function Page() {
  return <MCP />;
}
