import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";

const title = "About DX";
const description =
  "About DX. Learn about the team and mission behind a unified development experience platform built for everyone.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/about`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/about`,
  },
};

const stats = [
  ["50,000+", "Developers using DX"],
  ["12ms", "Average startup time"],
  ["50+", "MCP apps available"],
  ["500+", "Keyboard shortcuts"],
  ["45MB", "RAM baseline"],
  ["100%", "Offline-capable workflows"],
];

const principles = [
  "Speed is not a feature. It's a requirement.",
  "Offline is not a fallback. It's first-class.",
  "AI should know your context, not ask for it repeatedly.",
  "Keyboard first. Mouse optional.",
  "Every tool should connect. Silos kill flow.",
  "Open protocols (MCP) over proprietary lock-in.",
];

const timeline = [
  "2023 Q1 · DX project started, Rust chosen as core runtime",
  "2023 Q3 · First alpha, 12ms startup milestone achieved",
  "2024 Q1 · Public beta launched",
  "2024 Q3 · MCP integrations released",
  "2024 Q4 · Offline local-model capability shipped",
  "2025 Q1 · 50,000 users, MCP app ecosystem scale-up",
  "2025 Q2 · Enterprise offering released",
];

export default function AboutPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8 space-y-10">
        <section className="border border-border p-6 sm:p-8">
          <p className="text-xs uppercase tracking-wide text-muted-foreground">About DX</p>
          <h1 className="mt-3 font-serif text-4xl text-foreground">We believe developers deserve better tools.</h1>
          <p className="mt-4 text-muted-foreground max-w-3xl">
            Every developer knows the pain: slow tooling, fragmented context, and assistants that forget your work every step.
            We built DX from the ground up in Rust to deliver instant, connected, intelligent workflows online or offline.
          </p>
        </section>

        <section className="border border-border p-6 sm:p-8">
          <h2 className="font-serif text-3xl text-foreground">By the Numbers</h2>
          <div className="mt-5 grid grid-cols-2 md:grid-cols-3 gap-4">
            {stats.map(([value, label]) => (
              <div key={label} className="border border-border p-4">
                <p className="text-2xl text-foreground">{value}</p>
                <p className="mt-1 text-sm text-muted-foreground">{label}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="border border-border p-6 sm:p-8">
          <h2 className="font-serif text-3xl text-foreground">What We Believe</h2>
          <div className="mt-5 grid grid-cols-1 md:grid-cols-2 gap-4">
            {principles.map((principle) => (
              <div key={principle} className="border border-border p-4 text-muted-foreground">
                {principle}
              </div>
            ))}
          </div>
        </section>

        <section className="border border-border p-6 sm:p-8">
          <h2 className="font-serif text-3xl text-foreground">Milestones</h2>
          <ul className="mt-5 space-y-3">
            {timeline.map((item) => (
              <li key={item} className="border border-border p-3 text-sm text-muted-foreground">
                {item}
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
