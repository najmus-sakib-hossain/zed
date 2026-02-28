import type { Metadata } from "next";
import Link from "next/link";
import { baseUrl } from "@/app/sitemap";

const title = "DX Blog";
const description = "Engineering, product, and workflow deep dives from the DX team.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/blog` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/blog` },
};

const posts = [
  "Why We Chose Rust for DX",
  "Introducing MCP Apps",
  "Complete Guide to DX Shortcuts",
  "How Offline Mode Works",
  "Token Saving in Practice",
  "Building Your First Custom MCP App",
];

export default function BlogPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">
        <h1 className="font-serif text-4xl text-foreground">DX Blog</h1>
        <p className="mt-3 text-muted-foreground">Engineering notes, release deep dives, and implementation guides.</p>

        <div className="mt-8 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {posts.map((titleItem) => (
            <article key={titleItem} className="border border-border p-4">
              <h2 className="text-foreground text-lg">{titleItem}</h2>
              <p className="mt-2 text-sm text-muted-foreground">Long-form technical article coming soon.</p>
              <Link href="/updates" className="mt-4 inline-block text-sm text-foreground underline underline-offset-4">
                Read latest updates
              </Link>
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}
