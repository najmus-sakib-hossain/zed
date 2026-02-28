import { Button } from "@midday/ui/button";
import type { Metadata } from "next";
import Link from "next/link";
import { baseUrl } from "@/app/sitemap";
import { competitors } from "@/data/competitors";

const year = new Date().getFullYear();
const title = `Compare DX to VS Code, Cursor, JetBrains & More (${year})`;
const description =
  "See how DX compares to other developer tools. Built on Rust, offline-first, with 100+ AI providers, 400+ connects, Forge VCS, and a unified media engine.";

export const metadata: Metadata = {
  title,
  description,
  keywords: [
    "DX alternative",
    "VS Code alternative",
    "Cursor alternative",
    "JetBrains alternative",
    "developer tools comparison",
    "AI developer environment",
    "Rust IDE",
    "offline AI coding",
  ],
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/compare`,
    images: [
      {
        url: `${baseUrl}/api/og/compare`,
        width: 1200,
        height: 630,
        alt: "Compare DX to alternatives",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
    images: [`${baseUrl}/api/og/compare`],
  },
  alternates: {
    canonical: `${baseUrl}/compare`,
  },
};

export default function ComparePage() {
  return (
    <div className="min-h-screen pt-24 sm:pt-28 lg:pt-32 pb-24">
      <div className="max-w-[1400px] mx-auto">
        {/* Header */}
        <div className="text-center mb-12 lg:mb-16">
          <h1 className="font-serif text-3xl lg:text-4xl text-foreground mb-4">
            Compare DX to alternatives
          </h1>
          <p className="font-sans text-base text-muted-foreground max-w-2xl mx-auto">
            DX is built for teams that want connected AI workflows across code,
            research, automation, and media without unnecessary complexity.
          </p>
        </div>

        {/* Competitors Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-16">
          {competitors.map((competitor) => (
            <Link
              key={competitor.id}
              href={`/compare/${competitor.slug}`}
              className="border border-border p-6 hover:border-foreground/20 transition-all duration-200"
            >
              <h2 className="font-sans text-lg text-foreground mb-2">
                {competitor.name} Alternative
              </h2>
              <p className="font-sans text-sm text-muted-foreground mb-4 line-clamp-2">
                {competitor.description}
              </p>
              <div className="flex flex-wrap gap-2">
                {competitor.keyDifferences.slice(0, 2).map((diff) => (
                  <span
                    key={diff.title}
                    className="font-sans text-xs text-muted-foreground bg-muted px-2 py-1"
                  >
                    {diff.midday}
                  </span>
                ))}
              </div>
            </Link>
          ))}
        </div>

        {/* CTA Section */}
        <div className="bg-background border border-border p-8 lg:p-12 text-center relative before:absolute before:inset-0 before:bg-[repeating-linear-gradient(-60deg,rgba(219,219,219,0.4),rgba(219,219,219,0.4)_1px,transparent_1px,transparent_6px)] dark:before:bg-[repeating-linear-gradient(-60deg,rgba(44,44,44,0.4),rgba(44,44,44,0.4)_1px,transparent_1px,transparent_6px)] before:pointer-events-none">
          <div className="relative z-10">
            <h2 className="font-serif text-2xl text-foreground mb-4">
              Ready to try DX?
            </h2>
            <p className="font-sans text-base text-muted-foreground mb-6 max-w-xl mx-auto">
              Start your free trial and see why teams are switching to DX.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Button asChild className="btn-inverse h-11 px-6">
                <a href="#waitlist">Start your free trial</a>
              </Button>
              <Button asChild variant="outline" className="h-11 px-6">
                <Link href="/pricing">View pricing</Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
