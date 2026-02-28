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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@midday/ui/table";
import { baseUrl } from "@/app/sitemap";

const title = "Forge — Unlimited Version Control for Every Media Type";
const description =
  "Forge gives you Git-level version history for video, images, audio, 3D/AR/VR, and code — storing every asset to the platforms that already give you unlimited free storage.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/forge` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/forge` },
};

const storageRoutes = [
  {
    type: "Video",
    platform: "YouTube (unlisted / draft)",
    limit: "Unlimited",
    why: "Every video iteration stored as a private draft. Never pay for storage.",
  },
  {
    type: "Images",
    platform: "Pinterest libraries",
    limit: "Unlimited",
    why: "Boards map naturally to branches — visual diffs at a glance.",
  },
  {
    type: "Audio",
    platform: "SoundCloud / Spotify-like",
    limit: "Unlimited",
    why: "Waveform timeline doubles as a version scrubber.",
  },
  {
    type: "3D / AR / VR",
    platform: "Sketchfab + Thingiverse",
    limit: "Unlimited",
    why: "Model diffs with scene-level metadata preserved.",
  },
  {
    type: "Code + Docs",
    platform: "GitHub / GitLab / Bitbucket (multi-target)",
    limit: "Unlimited",
    why: "Push to all three simultaneously — one commit, full redundancy.",
  },
];

const tiers = [
  {
    name: "Free Tier",
    badge: "Free Forever",
    badgeClass: "bg-green-500/10 text-green-600 dark:text-green-400",
    price: "$0",
    items: [
      "YouTube, Pinterest, SoundCloud, Sketchfab, GitHub",
      "Unlimited storage via platform-native backends",
      "Full VCS history, rollback, and branch support",
      "Works 100% offline (Rust-powered local caching)",
    ],
    cta: { label: "Get Started Free", href: "/download" },
    highlight: false,
  },
  {
    name: "Pro Storage",
    badge: "Recommended for studios",
    badgeClass: "bg-primary/10 text-primary",
    price: "$19/mo",
    items: [
      "Cloudflare R2 private bucket (direct DX integration)",
      "Google Drive, Dropbox, Mega, and more",
      "End-to-end encryption for all stored assets",
      "Private collaboration with fine-grained permissions",
    ],
    cta: { label: "Start Pro Trial", href: "/download" },
    highlight: true,
  },
  {
    name: "All Cloud Storage",
    badge: "Enterprise",
    badgeClass: "bg-secondary text-secondary-foreground",
    price: "Custom",
    items: [
      "All Free + Pro backends simultaneously",
      "Custom CDN and on-prem storage connectors",
      "Compliance controls (SOC2, GDPR, HIPAA-ready)",
      "Priority sync and dedicated bandwidth pools",
    ],
    cta: { label: "Contact Sales", href: "/contact" },
    highlight: false,
  },
];

const playbook = [
  {
    step: "01",
    title: "Generate assets in one DX flow",
    detail:
      "Create video, image, audio, 3D, or code inside DX using any connected AI provider or your own tools.",
  },
  {
    step: "02",
    title: "Route each media type to its platform",
    detail:
      "Forge automatically classifies assets and pushes each to the correct backend — no manual upload steps.",
  },
  {
    step: "03",
    title: "Version every iteration",
    detail:
      "Every change is hashed, timestamped, and indexed. Browse history, diff outputs, and restore any version.",
  },
  {
    step: "04",
    title: "Promote from draft to publish-ready",
    detail:
      "Merge branches, tag releases, and push to public channels directly from the Forge interface inside DX.",
  },
];

const faqs = [
  {
    q: "I don't trust my media on public platforms. Can I keep it private?",
    a: "Yes. YouTube unlisted drafts, Pinterest secret boards, and SoundCloud private tracks are invisible to anyone without your link. The Pro tier adds R2/Drive/Dropbox with encryption for full privacy.",
  },
  {
    q: "What if YouTube changes its API?",
    a: "Forge uses stable v3 Data API scopes. If a backend changes, DX alerts you and migrates assets to your next-best available backend without losing history.",
  },
  {
    q: "Can I push to GitHub and GitLab at the same time?",
    a: "Yes. Multi-target push is a first-class feature — one `forge push` command commits to all linked code platforms simultaneously.",
  },
  {
    q: "Does Forge work offline?",
    a: "Fully. Forge queues all commits locally in the DX Rust runtime and syncs when connectivity returns. No data is lost during disconnected sessions.",
  },
];

export default function ForgePage() {
  return (
    <div className="min-h-screen pt-24 sm:pt-28 pb-24">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">

        {/* Hero */}
        <div className="mb-16">
          <Badge variant="outline" className="mb-4">Built into DX</Badge>
          <h1 className="font-serif text-3xl sm:text-4xl lg:text-5xl text-foreground mb-4 max-w-3xl">
            Forge: Unlimited VCS for Every Media Type
          </h1>
          <p className="text-base sm:text-lg text-muted-foreground max-w-2xl mb-8">
            {description}
          </p>
          <div className="flex flex-col sm:flex-row gap-3">
            <Button asChild className="btn-inverse h-11 px-6">
              <Link href="/download">Get Forge Free</Link>
            </Button>
            <Button asChild variant="outline" className="h-11 px-6">
              <Link href="/docs/workflows">Read the Docs</Link>
            </Button>
          </div>
        </div>

        <Separator className="mb-16" />

        {/* Storage Routes Table */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-2">
            Zero-cost storage, platform-native
          </h2>
          <p className="text-sm text-muted-foreground mb-6">
            Every media type maps to a platform that already gives you unlimited free storage. No credit card required for the core tier.
          </p>
          <div className="border border-border overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Media type</TableHead>
                  <TableHead>Storage platform</TableHead>
                  <TableHead>Limit</TableHead>
                  <TableHead>Why it works</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {storageRoutes.map((row) => (
                  <TableRow key={row.type}>
                    <TableCell className="font-medium text-foreground">{row.type}</TableCell>
                    <TableCell className="text-foreground">{row.platform}</TableCell>
                    <TableCell>
                      <Badge variant="tag" className="bg-green-500/10 text-green-600 dark:text-green-400 border-none">
                        {row.limit}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-sm">{row.why}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </section>

        {/* Tier Cards */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-6">Choose your storage tier</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            {tiers.map((tier) => (
              <Card
                key={tier.name}
                className={tier.highlight ? "ring-2 ring-primary" : ""}
              >
                <CardHeader>
                  <div className="flex items-center justify-between mb-1">
                    <CardTitle className="text-base">{tier.name}</CardTitle>
                    <Badge className={tier.badgeClass} variant="outline">
                      {tier.badge}
                    </Badge>
                  </div>
                  <p className="text-2xl font-semibold text-foreground">{tier.price}</p>
                </CardHeader>
                <CardContent className="space-y-3">
                  <ul className="space-y-2">
                    {tier.items.map((item) => (
                      <li key={item} className="flex items-start gap-2 text-sm text-muted-foreground">
                        <span className="mt-0.5 shrink-0 text-foreground">✓</span>
                        {item}
                      </li>
                    ))}
                  </ul>
                  <Separator />
                  <Button asChild variant={tier.highlight ? "default" : "outline"} className="w-full h-9">
                    <Link href={tier.cta.href}>{tier.cta.label}</Link>
                  </Button>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        <Separator className="mb-16" />

        {/* How it works */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-6">How Forge works</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {playbook.map((step) => (
              <Card key={step.step} className="p-5">
                <p className="text-xs uppercase tracking-wider text-muted-foreground mb-2">Step {step.step}</p>
                <p className="font-medium text-foreground mb-1">{step.title}</p>
                <p className="text-sm text-muted-foreground">{step.detail}</p>
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
