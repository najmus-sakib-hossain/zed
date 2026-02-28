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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@midday/ui/tabs";
import { baseUrl } from "@/app/sitemap";

const title = "DX Media — Unified Assets, Editing, and Version Control";
const description =
  "One media layer for every asset in your workflow: fetch from 20+ providers, edit inline, track versions, manage 5,000+ fonts and 1M+ icons, and collaborate in real time.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/media` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/media` },
};

const stats = [
  { value: "5,000+", label: "Fonts" },
  { value: "219+", label: "Icon sets" },
  { value: "1,000,000+", label: "Icons" },
  { value: "20+", label: "Fetch providers" },
];

const fetchProviders = {
  Images: ["Unsplash", "Pexels", "Pixabay", "Custom providers"],
  Video: ["YouTube", "Vimeo", "Custom providers"],
  Audio: ["Spotify", "SoundCloud", "Custom providers"],
  "3D / AR / VR": ["Sketchfab", "Thingiverse", "Custom providers"],
};

const editorCaps = [
  { title: "Image editing", items: ["Crop & resize", "Colour correction", "Filter pipeline", "Batch export"] },
  { title: "Audio editing", items: ["Trim & loop", "Volume curves", "Multi-track mix", "Normalise"] },
  { title: "Video editing", items: ["Trim & splice", "Subtitle burn-in", "Thumbnail gen", "Compress & convert"] },
  { title: "3D editing", items: ["Mesh inspection", "Texture swap", "Scene metadata", "Format convert"] },
];

const faqs = [
  {
    q: "Can I bring my own font files?",
    a: "Yes. Upload any OTF, TTF, or WOFF2 and it becomes instantly searchable and available across all your DX projects.",
  },
  {
    q: "How does media version control differ from Forge?",
    a: "Media version control inside the Media module tracks editing changes within a session. Forge handles cross-platform storage and long-term VCS. Both are linked — a Forge commit captures the Media version snapshot.",
  },
  {
    q: "Is the media library shared across team members?",
    a: "Yes, with permission levels: view-only, comment, and edit. Real-time collaborative edits use CRDT-based conflict resolution.",
  },
];

export default function MediaPage() {
  return (
    <div className="min-h-screen pt-24 sm:pt-28 pb-24">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">

        {/* Hero */}
        <div className="mb-16">
          <Badge variant="outline" className="mb-4">Built into DX</Badge>
          <h1 className="font-serif text-3xl sm:text-4xl lg:text-5xl text-foreground mb-4 max-w-3xl">
            DX Media: Every Asset, One Layer
          </h1>
          <p className="text-base sm:text-lg text-muted-foreground max-w-2xl mb-8">
            {description}
          </p>
          <div className="flex flex-col sm:flex-row gap-3">
            <Button asChild className="btn-inverse h-11 px-6">
              <Link href="/download">Start Using DX Media</Link>
            </Button>
            <Button asChild variant="outline" className="h-11 px-6">
              <Link href="/docs/getting-started">Read the Docs</Link>
            </Button>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-16">
          {stats.map((s) => (
            <Card key={s.label} className="p-6 text-center">
              <p className="text-2xl sm:text-3xl font-semibold text-foreground mb-1">{s.value}</p>
              <p className="text-sm text-muted-foreground">{s.label}</p>
            </Card>
          ))}
        </div>

        <Separator className="mb-16" />

        {/* Fetch Providers */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-2">Fetch from anywhere</h2>
          <p className="text-sm text-muted-foreground mb-6">
            Pull assets directly into your DX workflow from 20+ built-in providers. Link custom providers via the Integrations panel.
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {Object.entries(fetchProviders).map(([category, providers]) => (
              <Card key={category}>
                <CardHeader>
                  <CardTitle className="text-base">{category}</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-1.5">
                    {providers.map((p) => (
                      <Badge key={p} variant="tag">{p}</Badge>
                    ))}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </section>

        <Separator className="mb-16" />

        {/* Editor Capabilities Tabs */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-6">Built-in editor</h2>
          <Tabs defaultValue="Image editing">
            <TabsList className="mb-4 flex-wrap h-auto gap-1">
              {editorCaps.map((cap) => (
                <TabsTrigger key={cap.title} value={cap.title}>{cap.title}</TabsTrigger>
              ))}
            </TabsList>
            {editorCaps.map((cap) => (
              <TabsContent key={cap.title} value={cap.title}>
                <Card>
                  <CardContent className="pt-6">
                    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                      {cap.items.map((item) => (
                        <div key={item} className="border border-border p-3 text-sm text-foreground">
                          {item}
                        </div>
                      ))}
                    </div>
                  </CardContent>
                </Card>
              </TabsContent>
            ))}
          </Tabs>
        </section>

        <Separator className="mb-16" />

        {/* Fonts + Icons */}
        <section className="mb-16">
          <h2 className="font-serif text-2xl text-foreground mb-4">Fonts &amp; Icons</h2>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <Card className="p-6">
              <p className="text-3xl font-semibold text-foreground mb-1">5,000+</p>
              <p className="text-sm font-medium text-foreground mb-1">Fonts</p>
              <p className="text-sm text-muted-foreground">Search, preview, and apply any font across all projects. Upload custom OTF/TTF/WOFF2 files.</p>
            </Card>
            <Card className="p-6">
              <p className="text-3xl font-semibold text-foreground mb-1">219+</p>
              <p className="text-sm font-medium text-foreground mb-1">Icon sets</p>
              <p className="text-sm text-muted-foreground">Includes Lucide, Heroicons, Radix, Phosphor, Material, and 214 more. All searchable in one place.</p>
            </Card>
            <Card className="p-6">
              <p className="text-3xl font-semibold text-foreground mb-1">1,000,000+</p>
              <p className="text-sm font-medium text-foreground mb-1">Icons</p>
              <p className="text-sm text-muted-foreground">Unified search across all sets. Export as SVG, React component, or image at any size.</p>
            </Card>
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
