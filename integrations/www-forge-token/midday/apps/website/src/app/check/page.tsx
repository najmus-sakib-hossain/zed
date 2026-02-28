import type { Metadata } from "next";
import Link from "next/link";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@midday/ui/accordion";
import { Badge } from "@midday/ui/badge";
import { Button } from "@midday/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@midday/ui/card";
import { Separator } from "@midday/ui/separator";
import { baseUrl } from "@/app/sitemap";

const title = "Check — 500-Point Security & Quality Scoring";
const description =
  "Check scans your codebase and media assets, scores them on a 500-point anime-style rank ladder (F to SSSSS), and gives you fix-ready recommendations to climb the ranks.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/check` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/check` },
};

const ranks = [
  { rank: "F", color: "text-red-500", bg: "bg-red-500/10", range: "0–49", note: "Critical issues, unusable structure" },
  { rank: "E", color: "text-orange-500", bg: "bg-orange-500/10", range: "50–99", note: "Major naming and security gaps" },
  { rank: "D", color: "text-amber-500", bg: "bg-amber-500/10", range: "100–149", note: "Below acceptable threshold" },
  { rank: "C", color: "text-yellow-500", bg: "bg-yellow-500/10", range: "150–199", note: "Passing but needs work" },
  { rank: "B", color: "text-lime-500", bg: "bg-lime-500/10", range: "200–249", note: "Good baseline hygiene" },
  { rank: "A", color: "text-green-500", bg: "bg-green-500/10", range: "250–299", note: "Production-ready standard" },
  { rank: "S", color: "text-teal-500", bg: "bg-teal-500/10", range: "300–349", note: "Senior-engineer quality" },
  { rank: "SS", color: "text-cyan-500", bg: "bg-cyan-500/10", range: "350–399", note: "Elite structure and security" },
  { rank: "SSS", color: "text-blue-500", bg: "bg-blue-500/10", range: "400–449", note: "Exceptional across all axes" },
  { rank: "SSSS", color: "text-violet-500", bg: "bg-violet-500/10", range: "450–474", note: "Near-perfect codebase" },
  { rank: "SSSSS", color: "text-purple-500", bg: "bg-purple-500/10", range: "475–500", note: "Legendary — 1% of projects" },
];

const scanners = [
  {
    title: "Security Vulnerability Scanner",
    desc: "Detects OWASP top-10 patterns, hardcoded secrets, insecure deps, and exposed API surfaces in code and media metadata.",
  },
  {
    title: "Code Linter",
    desc: "Enforces naming conventions, structural consistency, unused exports, and complexity thresholds across all supported languages.",
  },
  {
    title: "Media Linter",
    desc: "Checks image/video metadata hygiene, oversized assets, missing alt text, and format inefficiencies that hurt performance.",
  },
  {
    title: "Security Audit Report",
    desc: "Full project-wide audit that outputs a prioritised, fix-ordered report with one-click jump-to-issue navigation inside DX.",
  },
];

const scoringAxes = [
  { axis: "File & folder naming", weight: "100 pts", example: "Clear, consistent naming conventions" },
  { axis: "Error handling & reliability", weight: "100 pts", example: "Typed errors, boundary coverage" },
  { axis: "Security patterns", weight: "100 pts", example: "No hardcoded secrets, safe external calls" },
  { axis: "Overall structure", weight: "100 pts", example: "Logical module separation, clean deps" },
  { axis: "Media & asset quality", weight: "100 pts", example: "Optimised sizes, correct formats" },
];

const faqs = [
  {
    q: "How quickly does a Check scan run?",
    a: "Rust-powered — typically under 3 seconds for projects up to 100k files. Incremental rescanning after edits takes under 500ms.",
  },
  {
    q: "Can I configure the scoring weights?",
    a: "Yes. You can customise weight budgets per axis via a `.dxcheck.toml` in your project root, or use one of the built-in profiles (Security-focused, Media-heavy, Code-only).",
  },
  {
    q: "Does Check run offline?",
    a: "All scanning and scoring runs locally. No file contents are ever sent to an external server. Only aggregated score metadata can optionally sync to your DX cloud profile.",
  },
];

export default function CheckPage() {
  return (
    <div className="min-h-screen pt-24 sm:pt-28 pb-24">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">

        {/* Hero */}
        <div className="mb-16">
          <Badge variant="outline" className="mb-4">Built into DX</Badge>
          <h1 className="font-serif text-3xl sm:text-4xl lg:text-5xl text-foreground mb-4 max-w-3xl">
            Check: 500-Point Security &amp; Quality Score
          </h1>
          <p className="text-base sm:text-lg text-muted-foreground max-w-2xl mb-8">
            {description}
          </p>
          <div className="flex flex-col sm:flex-row gap-3">
            <Button asChild className="btn-inverse h-11 px-6">
              <Link href="/download">Run Your First Check</Link>
            </Button>
            <Button asChild variant="outline" className="h-11 px-6">
              <Link href="/docs/getting-started">Read the Docs</Link>
            </Button>
          </div>
        </div>

        <Separator className="mb-16" />

        {/* Rank Ladder */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-2">The rank ladder</h2>
          <p className="text-sm text-muted-foreground mb-6">
            11 ranks across a 500-point scale. Every project starts at F. Work your way to SSSSS.
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
            {ranks.map((r) => (
              <Card key={r.rank} className={`p-4 ${r.bg}`}>
                <p className={`text-2xl font-bold mb-1 ${r.color}`}>{r.rank}</p>
                <p className="text-xs text-muted-foreground font-medium">{r.range} / 500</p>
                <p className="text-xs text-muted-foreground mt-1">{r.note}</p>
              </Card>
            ))}
          </div>
        </section>

        <Separator className="mb-16" />

        {/* Scoring Axes */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-6">What gets scored</h2>
          <div className="border border-border divide-y divide-border">
            {scoringAxes.map((row) => (
              <div key={row.axis} className="flex flex-col sm:flex-row sm:items-center gap-2 px-5 py-4">
                <div className="flex-1">
                  <p className="text-sm font-medium text-foreground">{row.axis}</p>
                  <p className="text-xs text-muted-foreground mt-0.5">{row.example}</p>
                </div>
                <Badge variant="outline" className="w-fit">{row.weight}</Badge>
              </div>
            ))}
          </div>
        </section>

        {/* Scanner Cards */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-6">Built-in scanners</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {scanners.map((s) => (
              <Card key={s.title}>
                <CardHeader>
                  <CardTitle className="text-base">{s.title}</CardTitle>
                </CardHeader>
                <CardContent>
                  <p className="text-sm text-muted-foreground">{s.desc}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        {/* FAQ */}
        <section>
          <h2 className="font-serif text-2xl text-foreground mb-6">Common questions</h2>
          <Accordion type="single" collapsible className="border border-border px-4">
            {faqs.map((faq, i) => (
              <AccordionItem key={faq.q} value={`faq-${i}`}>
                <AccordionTrigger className="text-sm text-foreground text-left">{faq.q}</AccordionTrigger>
                <AccordionContent className="text-sm text-muted-foreground">{faq.a}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </section>
      </div>
    </div>
  );
}
