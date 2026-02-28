import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxVideoCarouselSections } from "@/components/dx-video-carousel-sections";
import Link from "next/link";

const title = "DX Documentation";
const description =
  "Technical documentation for DX architecture, connected workflows, token-saving systems, offline capability, and integration runtime behavior.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/docs`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/docs`,
  },
};

export default function DocsPage() {
  return (
    <>
      <section className="pt-28 pb-8 border-b border-border bg-background">
        <div className="mx-auto w-full max-w-6xl px-6">
          <div className="space-y-2 mb-8">
            <h1 className="text-3xl sm:text-4xl font-semibold tracking-tight text-foreground">
              DX Documentation
            </h1>
            <p className="text-muted-foreground max-w-3xl">
              Implementation-first docs for setup, shortcuts, workflows, MCP apps, offline mode, and API integration.
            </p>
          </div>

          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {[
              { href: "/docs/getting-started", label: "Getting Started" },
              { href: "/docs/shortcuts", label: "Shortcuts" },
              { href: "/docs/workflows", label: "Workflows" },
              { href: "/docs/mcp-apps", label: "MCP Apps" },
              { href: "/docs/offline", label: "Offline" },
              { href: "/docs/api", label: "API" },
            ].map((topic) => (
              <Link
                key={topic.href}
                href={topic.href}
                className="rounded-lg border border-border bg-card/40 px-4 py-3 text-sm text-muted-foreground hover:text-foreground hover:bg-card transition-colors"
              >
                {topic.label}
              </Link>
            ))}
          </div>
        </div>
      </section>

      <DxVideoCarouselSections
        pageTitle="DX Documentation"
        pageDescription="Documentation in DX is implementation-first: architecture, token pipeline internals, connected execution, and reproducible workflow patterns."
      />
    </>
  );
}
