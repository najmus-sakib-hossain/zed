import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { MCPMake } from "@/components/mcp-make";

const title = "Make MCP Integration";
const description =
  "Connect DX to Make scenarios via MCP. Build visual automations with your data and connect to 1,500+ apps.";

export const metadata: Metadata = {
  title,
  description,
  keywords: [
    "Make MCP",
    "Make integration",
    "Model Context Protocol",
    "visual automation",
    "no-code workflows",
  ],
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/mcp/make`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/mcp/make`,
  },
};

export default function Page() {
  return <MCPMake />;
}
