import type { Metadata } from "next";
import Link from "next/link";
import { baseUrl } from "@/app/sitemap";

const title = "DX Changelog";
const description = "Latest DX releases, improvements, and fixes.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/changelog` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/changelog` },
};

const entries = [
  {
    version: "v2.5.0",
    date: "March 20, 2025",
    items: [
      "MCP App Store rollout and install flow",
      "Workflow templates and Vim mode improvements",
      "40% faster indexing and offline sync reliability upgrades",
    ],
  },
  {
    version: "v2.4.0",
    date: "February 28, 2025",
    items: [
      "200 new keyboard shortcuts",
      "Local LLM support upgrades",
      "Automation trigger updates",
    ],
  },
];

export default function ChangelogPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[900px] mx-auto px-4 sm:px-8">
        <h1 className="font-serif text-4xl text-foreground">What&apos;s New</h1>
        <p className="mt-3 text-muted-foreground">Timeline of DX releases and platform updates.</p>

        <div className="mt-8 space-y-5">
          {entries.map((entry) => (
            <article key={entry.version} className="border border-border p-5">
              <h2 className="text-foreground text-xl">{entry.version}</h2>
              <p className="text-sm text-muted-foreground mt-1">{entry.date}</p>
              <ul className="mt-3 space-y-2 text-sm text-muted-foreground">
                {entry.items.map((item) => (
                  <li key={item}>• {item}</li>
                ))}
              </ul>
            </article>
          ))}
        </div>

        <div className="mt-8 text-sm">
          <Link href="/updates" className="text-foreground underline underline-offset-4">View all update posts</Link>
        </div>
      </div>
    </div>
  );
}
