import type { Metadata } from "next";
import { Badge } from "@midday/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@midday/ui/card";
import { Separator } from "@midday/ui/separator";
import { baseUrl } from "@/app/sitemap";

const title = "DX — Loved by Developers Everywhere";
const description =
  "From solo engineers to enterprise teams — developers across every OS and every stack trust DX to ship faster, stay in flow, and eliminate tool friction.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/customers`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/customers`,
  },
};

const stats = [
  { value: "50,000+", label: "Developers" },
  { value: "100+", label: "LLM Providers" },
  { value: "400+", label: "Connects" },
  { value: "30–90%", label: "Token Savings" },
];

const testimonials = [
  {
    quote: "DX replaced four separate tools I was juggling. The offline mode alone is worth the switch.",
    author: "Senior Engineer, Series B startup",
    tags: ["Offline", "Rust"],
  },
  {
    quote: "Forge is the first version control for media that actually makes sense. My team went fully on it in a week.",
    author: "Creative Director, agency",
    tags: ["Forge", "Media VCS"],
  },
  {
    quote: "The Check score system caught naming issues and security gaps I'd been ignoring. S-rank changed how I review PRs.",
    author: "Staff Engineer, fintech",
    tags: ["Check", "Security"],
  },
  {
    quote: "I can run 6 modes from the same session — Ask to Research to Agent handoff — without losing context.",
    author: "AI engineer, LLM startup",
    tags: ["Assistant", "Modes"],
  },
  {
    quote: "Token savings are real. We cut our AI API bill by 60% in the first month.",
    author: "CTO, developer tools company",
    tags: ["RLM", "Cost"],
  },
  {
    quote: "Works on my Mac, my Linux box, and even my Android tablet. First tool that truly goes everywhere I do.",
    author: "Indie developer",
    tags: ["Cross-platform", "Mobile"],
  },
];

export default function Page() {
  return (
    <div className="min-h-screen pt-24 sm:pt-28 pb-24">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">
        {/* Hero */}
        <div className="text-center mb-12">
          <Badge variant="outline" className="mb-4">Developer Stories</Badge>
          <h1 className="font-serif text-3xl sm:text-4xl text-foreground mb-4">
            Loved by developers everywhere
          </h1>
          <p className="text-base text-muted-foreground max-w-2xl mx-auto">
            {description}
          </p>
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

        {/* Testimonials */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {testimonials.map((t) => (
            <Card key={t.author} className="flex flex-col">
              <CardContent className="pt-6 flex-1">
                <p className="text-sm text-foreground leading-relaxed">&ldquo;{t.quote}&rdquo;</p>
              </CardContent>
              <CardHeader className="pt-2">
                <CardTitle className="text-xs text-muted-foreground font-normal">{t.author}</CardTitle>
                <div className="flex flex-wrap gap-1.5 mt-2">
                  {t.tags.map((tag) => (
                    <Badge key={tag} variant="tag">{tag}</Badge>
                  ))}
                </div>
              </CardHeader>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
