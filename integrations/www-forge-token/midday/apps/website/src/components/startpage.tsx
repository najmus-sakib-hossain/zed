"use client";

import { useGSAP } from "@gsap/react";
import Link from "next/link";
import Image from "next/image";
import { Badge } from "@midday/ui/badge";
import { Button } from "@midday/ui/button";
import { Card, CardContent } from "@midday/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@midday/ui/table";
import { Separator } from "@midday/ui/separator";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { useRef } from "react";
import { DxAiFace } from "./dx-ai-face";
import { DxVideoShowcases } from "./dx-video-showcases";
import { PlatformDownloadCards } from "./platform-download-cards";

gsap.registerPlugin(ScrollTrigger);

const THUMBS = [
  "/thumbnails/amber.png",
  "/thumbnails/blue.png",
  "/thumbnails/cyan.png",
  "/thumbnails/emerald.png",
  "/thumbnails/fuchsia.png",
  "/thumbnails/green-variant.png",
  "/thumbnails/green.png",
  "/thumbnails/indigo.png",
  "/thumbnails/lime.png",
  "/thumbnails/orange.png",
  "/thumbnails/pink.png",
  "/thumbnails/purple.png",
  "/thumbnails/rainbow.png",
  "/thumbnails/red.png",
  "/thumbnails/rose.png",
  "/thumbnails/sky.png",
  "/thumbnails/teal.png",
  "/thumbnails/variant-1.png",
  "/thumbnails/variant-2.png",
  "/thumbnails/violet.png",
  "/thumbnails/yellow.png",
] as const;
const thumb = (i: number): string => THUMBS[i % THUMBS.length] as string;

const marqueeCompanies = [
  "Arcforge",
  "Neonstack",
  "Byteplane",
  "Graphloom",
  "Shellgrid",
  "Nodecraft",
  "Cloudmesh",
  "Signalbase",
];

const featureCards = [
  {
    title: "Rust-Powered Performance",
    body: "Built from the ground up in Rust. 12ms startup. 45MB RAM baseline. 60fps UI under load.",
  },
  {
    category: "Docs & Files",
    capability: "PDFs, specs, reports, and document generation",
  },
  {
    category: "Charts & Data",
    capability: "Visualizations, dashboards, and analysis",
  },
  {
    title: "Offline-First",
    body: "Full capability without internet using local models, cached docs, and local workflows.",
  },
  {
    category: "Tool Calling",
    capability:
      "Full support for MCP, ACP, and A2A protocols (DX DCP-compatible)",
  },
  {
    category: "Images",
    capability:
      "Image generation, editing, and token-efficient image workflows",
  },
  {
    title: "MCP Apps Integration",
    body: "Native MCP app orchestration gives DX direct access to the tools that power your stack.",
  },
  {
    category: "3D",
    capability: "3D / AR / VR asset and scene generation",
  },
  {
    category: "Audio & Music",
    capability: "Sound design, composition, and voice synthesis",
  },
  {
    category: "Conversation",
    capability: "Real-time voice interaction",
  },
];

const platforms = [
  ["macOS", "Native Desktop App", "✅ Launch"],
  ["Windows", "Native Desktop App", "✅ Launch"],
  ["Linux", "Native Desktop App", "✅ Launch"],
  ["Android", "Mobile App", "✅ Launch"],
  ["iOS", "Mobile App", "✅ Launch"],
  ["ChromeOS", "Native / Web App", "✅ Launch"],
  ["Tablets", "Tablet UI", "✅ Launch"],
  ["watchOS", "Companion App", "✅ Launch"],
  ["tvOS", "Companion App", "✅ Launch"],
  ["Browser", "Remote Web Console + Extension", "✅ Launch"],
  ["IDEs/Editors", "Extensions", "✅ Launch"],
  ["Video Editors", "Plugins", "✅ Launch"],
  ["Image Editors", "Plugins", "✅ Launch"],
  ["Design Tools", "Plugins (Figma, Photoshop, etc.)", "✅ Launch"],
];

const comparisons = [
  ["Core Language", "Rust + GPUI", "Node.js / Electron"],
  [
    "Token Efficiency",
    "30–90% savings (RLM + tokenizers)",
    "No end-to-end optimization",
  ],
  ["Serialization", "DX Serializer (70–90% savings)", "Raw JSON payloads"],
  ["Offline Support", "Unlimited, free", "Internet + paid tiers"],
  [
    "AI Provider Support",
    "100+ providers + local models",
    "Locked to 1–3 providers",
  ],
  ["Connectors", "400+ connects + Cloud CLI skills", "Limited integrations"],
  [
    "Media Generation",
    "Text, images, video, 3D/AR/VR, audio + docs",
    "Mostly code only",
  ],
  [
    "Traffic Security",
    "Green / Yellow / Red automation + safe backups",
    "Manual review or all-or-nothing",
  ],
  [
    "Platform Coverage",
    "Desktop + mobile + ChromeOS + companion OS + extensions",
    "1–2 platforms, limited plugins",
  ],
];

const generationCategories = [
  { category: "Text & Code", capability: "Code generation, completion, refactor, and review" },
  { category: "Images", capability: "Image generation, editing, and token-efficient image workflows" },
  { category: "Video", capability: "Video generation and processing pipelines" },
  { category: "Audio & Music", capability: "Sound design, composition, and voice synthesis" },
  { category: "3D / AR / VR", capability: "3D/AR/VR asset and scene generation" },
  { category: "Documents & PDFs", capability: "PDFs, specs, reports, and document generation" },
  { category: "Charts & Data", capability: "Visualizations, dashboards, and analysis" },
  { category: "Tool Calling", capability: "Full support for MCP, ACP, and A2A protocols (DX DCP-compatible)" },
  { category: "Conversation", capability: "Real-time voice interaction and STT/TTS" },
];

const testimonials = [
  {
    quote: "DX cut our token costs by 60% on day one. The RLM compression alone was worth the switch.",
    by: "Lena M., Staff Engineer @ Arcforge",
  },
  {
    quote: "Finally an AI tool that works offline. Deployed DX on a remote rig with no internet — full capability.",
    by: "Riku S., DevOps Lead @ Shellgrid",
  },
  {
    quote: "Forge is underrated. We now version all design exports alongside code with zero extra tooling.",
    by: "Priya K., Senior Developer @ Byteplane",
  },
];

const comparisonRows: string[][] = [
  ["Core Language", "Rust + GPUI", "TypeScript / Node.js", "Java / Kotlin"],
  ["Startup Time", "12ms", "~1.2s", "~3.5s"],
  ["RAM Baseline", "45MB", "200–800MB", "400MB–2GB"],
  ["AI Providers", "100+ + local models", "1–5 providers", "1–2 providers"],
  ["Offline AI", "Unlimited free local models", "No", "No"],
  ["Token Efficiency", "RLM + Serializer 30–90% savings", "None", "None"],
  ["Media Generation", "Text/image/video/3D/audio", "Code only", "Code only"],
  ["Traffic Security", "Green/Yellow/Red system", "Manual review", "Manual review"],
  ["VCS Coverage", "Code + all media types (Forge)", "Code only", "Code only"],
  ["Platforms", "9+ native + browser + extensions", "Desktop + browser ext", "Desktop only"],
  ["Connectors", "400+ connects", "Limited extensions", "Limited plugins"],
];

const stats = [
  { label: "LLM Providers", value: 100, suffix: "+" },
  { label: "Connects", value: 400, suffix: "+" },
  { label: "Token Savings", value: 90, suffix: "%" },
  { label: "Icon Library", value: 1000000, suffix: "+" },
];

const forgeRoutes = [
  ["Video", "YouTube unlisted/draft"],
  ["Images", "Pinterest libraries"],
  ["Audio", "SoundCloud/Spotify-like platforms"],
  ["3D/AR/VR", "Sketchfab-like storage endpoints"],
  ["Code + Docs", "GitHub / GitLab / Bitbucket (single or multi-target)"],
];

const trafficLevels = [
  {
    level: "Green",
    behavior: "Auto-executes harmless actions",
    detail: "Optimized for zero-friction flow when the operation is safe.",
  },
  {
    level: "Yellow",
    behavior: "Executes with warnings or silent notifications",
    detail:
      "Keeps velocity high while signaling medium-risk operations clearly.",
  },
  {
    level: "Red",
    behavior: "Executes with high-visibility warnings + safety backup",
    detail:
      "For destructive operations, DX creates a backup snapshot before changes.",
  },
];

const checkRanks = ["F", "E", "D", "C", "B", "A", "S"];

const mediaPillars = [
  "Unified support for audio, video, image, and 3D/AR/VR providers",
  "Built-in media library with organization and collaborative workflows",
  "5000+ fonts, 219+ icon sets and 1M+ icons, Images, Video, Audio, 3d assets and much more...",
  "Version control for media assets with rollback support",
  "Custom provider linking and reusable asset workflows",
];

const heroFeatures = [
  {
    label: "Forge",
    headline: "Unlimited Free Storage for Every Media Type",
    sub: "Video → YouTube · Images → Pinterest · Audio → SoundCloud · 3D → Sketchfab · Code → GitHub",
  },
  {
    label: "Traffic Security",
    headline: "AI That Acts Autonomously — Without Compromising Safety",
    sub: "Green auto-execute · Yellow warn · Red backup-then-execute · Auto-hash · Built-in Firewall & VPN",
  },
  {
    label: "Check",
    headline: "500-Point Anime Rank System for Every Project",
    sub: "F → SSSSS · Security scanner · Code linter · Media linter · Full audit reports",
  },
  {
    label: "Media Engine",
    headline: "5,000+ Fonts · 1M+ Icons · 20+ Providers · Built-in Editor",
    sub: "Fetch from Unsplash, Pexels, YouTube, Vimeo, Spotify, Sketchfab — edit, version, and collaborate",
  },
  {
    label: "Works Everywhere",
    headline: "9+ Native Platforms · Every Browser · Every IDE · Every Creative Tool",
    sub: "macOS · Windows · Linux · Android · iOS · ChromeOS · watchOS · tvOS · Remote Web Console",
  },
  {
    label: "100+ AI Providers",
    headline: "Any Model. Any Provider. Even Offline.",
    sub: "More providers than any competitor · Unlimited free local models · Hybrid cloud/offline switching",
  },
];

const builtInTools = [
  {
    name: "Workspace",
    desc: "Maintain a clean, organized workspace across any IDE. Consistent file layout, context sharing, and project scaffolding.",
  },
  {
    name: "Serializer",
    desc: "The most human-readable, most token-efficient, and fastest serializer in the world. 70–90% smaller payloads than JSON.",
  },
  {
    name: "i18n",
    desc: "Translate any text to any language. Speech-to-text and text-to-speech — free and unlimited.",
  },
  {
    name: "Driven",
    desc: "Maintain spec-driven AI workflows instead of vibe-coding AI slop. Deterministic, reproducible, auditable.",
  },
  {
    name: "DCP",
    desc: "Like MCP, A2A, and ACP — but more token-efficient, faster, and better. Drop-in protocol for agent communication.",
  },
];

const extensionsList = [
  { category: "Browsers", items: "Chrome, Safari, Firefox, Edge, Arc, Brave, Opera" },
  { category: "IDEs & Editors", items: "VS Code, JetBrains (IntelliJ, WebStorm, PyCharm), Neovim, Zed" },
  { category: "Design Tools", items: "Figma, Adobe Photoshop, Adobe Illustrator, Sketch, Canva" },
  { category: "Video Editors", items: "DaVinci Resolve, Adobe Premiere Pro, Final Cut Pro" },
  { category: "Communication", items: "WhatsApp, Telegram, Discord, Slack, Microsoft Teams" },
];

type PillarKey = "forge" | "traffic" | "check" | "media";

const pillarPlaybooks: Record<PillarKey, string[]> = {
  forge: [
    "Generate assets in one DX flow",
    "Route each media type to platform-specific storage",
    "Version every iteration and preserve rollback history",
    "Promote from draft to publish-ready channels",
  ],
  traffic: [
    "Classify action risk in real time",
    "Apply Green/Yellow/Red execution policy",
    "Protect sensitive values before outbound calls",
    "Create safety snapshot on high-risk operations",
  ],
  check: [
    "Scan structure, naming, and dependency hygiene",
    "Run security and vulnerability checks",
    "Score quality on a 500-point rank ladder",
    "Emit prioritized, fix-ready recommendations",
  ],
  media: [
    "Pull assets from linked providers",
    "Edit and organize in a shared media library",
    "Track versions and collaborator changes",
    "Export back to workflow tools with context intact",
  ],
};

const pillarVideos: Record<PillarKey, string[]> = {
  forge: [
    "Forge: Cross-platform media commit",
    "Forge: YouTube draft pipeline",
    "Forge: Multi-target publish flow",
  ],
  traffic: [
    "Traffic: Green auto-execution",
    "Traffic: Yellow warning path",
    "Traffic: Red safety snapshot",
  ],
  check: [
    "Check: Rank score walkthrough",
    "Check: Security audit run",
    "Check: Fix recommendation flow",
  ],
  media: [
    "Media: Multi-provider asset fetch",
    "Media: Collaborative edit loop",
    "Media: Version rollback + export",
  ],
};

const storyRail = [
  {
    step: "01",
    title: "Rust Core + GPUI Runtime",
    metric: "Up to 70% lower RAM pressure",
    detail:
      "DX runs heavy generation and orchestration without the Electron overhead that slows traditional AI tools.",
  },
  {
    step: "02",
    title: "Token Stack: RLM + Serializer + Tokenizers",
    metric: "30–90% token savings",
    detail:
      "Context compression and transport optimization compound across multi-step workflows to reduce cost and latency.",
  },
  {
    step: "03",
    title: "Always-On Workflows (Online + Offline)",
    metric: "No offline lockout",
    detail:
      "Switch between cloud and local execution paths while preserving workflow state and output continuity.",
  },
  {
    step: "04",
    title: "Scale Layer: 100+ Providers + 400+ Connects",
    metric: "One connected runtime",
    detail:
      "Models, tools, communication apps, and media pipelines share context so work moves end-to-end faster.",
  },
];

const deepDiveRoutes = [
  {
    title: "Assistant Flows",
    description:
      "See multi-step execution with connected context across Ask, Agent, Plan, Search, Study, and Research.",
    href: "/assistant",
  },
  {
    title: "MCP + AI Integrations",
    description:
      "Explore how DX routes business data and tools into Claude, Cursor, ChatGPT, Copilot, Raycast, Zapier, and more.",
    href: "/mcp",
  },
  {
    title: "Integrations Grid",
    description:
      "Review the integration surface and categories, then jump into provider-level detail pages.",
    href: "/integrations",
  },
  {
    title: "Documentation",
    description:
      "Read operational guidance for DX workflows, setup patterns, and production usage.",
    href: "/docs",
  },
];

const fadeIn = {
  initial: { opacity: 0, y: 18 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true, amount: 0.2 },
  transition: { duration: 0.45 },
};

function Section({
  id,
  title,
  subtitle,
  children,
}: {
  id: string;
  title: string;
  subtitle?: string;
  children: ReactNode;
}) {
  return (
    <motion.section
      id={id}
      className="py-16 sm:py-20 border-t border-border"
      {...fadeIn}
    >
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8">
        <h2 className="font-serif text-2xl sm:text-3xl text-foreground">
          {title}
        </h2>
        {subtitle ? (
          <p className="mt-3 text-base text-muted-foreground max-w-3xl">
            {subtitle}
          </p>
        ) : null}
        <div className="mt-8">{children}</div>
      </div>
    </motion.section>
  );
}

function VideoPlaceholderStrip({
  title,
  items,
  startIndex = 0,
}: {
  title: string;
  items: string[];
  startIndex?: number;
}) {
  const trackRef = useRef<HTMLDivElement>(null);

  const scrollByViewport = (direction: "prev" | "next") => {
    const track = trackRef.current;
    if (!track) return;
    const amount = track.clientWidth * 0.92;
    track.scrollBy({
      left: direction === "next" ? amount : -amount,
      behavior: "smooth",
    });
  };

  return (
    <div className="mt-6 border border-border p-4 sm:p-5">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-wide text-muted-foreground">
          {title}
        </p>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            className="h-8 px-3"
            onClick={() => scrollByViewport("prev")}
          >
            Prev
          </Button>
          <Button
            type="button"
            variant="outline"
            className="h-8 px-3"
            onClick={() => scrollByViewport("next")}
          >
            Next
          </Button>
        </div>
      </div>

      <div
        ref={trackRef}
        className="mt-4 flex gap-3 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      >
        {items.map((item, itemIndex) => (
          <div
            key={item}
            className="dx-video-card min-w-[260px] sm:min-w-[320px] md:min-w-[360px] border border-border overflow-hidden"
          >
            <div className="relative h-36 sm:h-40">
              <Image
                src={thumb(startIndex + itemIndex)}
                alt={item}
                fill
                className="object-cover"
                sizes="(max-width: 640px) 260px, (max-width: 768px) 320px, 360px"
                quality={95}
                priority={itemIndex < 3}
              />
              <div className="absolute inset-0 bg-black/30 flex items-center justify-center">
                <div className="w-12 h-12 rounded-full border-2 border-white/80 flex items-center justify-center">
                  <span className="text-white text-lg ml-0.5">▶</span>
                </div>
              </div>
            </div>
            <div className="p-3 bg-background">
              <p className="text-sm text-foreground">{item}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function FeatureHeroBanner({
  label,
  headline,
  sub,
  imgIndex = 0,
}: {
  label: string;
  headline: string;
  sub: string;
  imgIndex?: number;
}) {
  return (
    <motion.section
      className="relative overflow-hidden my-8"
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, amount: 0.2 }}
      transition={{ duration: 0.5 }}
    >
      <div className="relative h-[320px] sm:h-[380px]">
        <Image
          src={thumb(imgIndex)}
          alt={headline}
          fill
          className="object-cover"
          sizes="100vw"
          quality={95}
          priority={imgIndex === 0}
        />
        <div className="absolute inset-0 bg-black/40" />
        <div className="absolute inset-0 flex flex-col items-center justify-center text-center px-6">
          <Badge variant="secondary" className="mb-4 text-xs uppercase tracking-wider">
            {label}
          </Badge>
          <h2 className="font-serif text-2xl sm:text-3xl lg:text-4xl text-white max-w-4xl leading-tight">
            {headline}
          </h2>
          <p className="mt-4 text-sm sm:text-base text-white/80 max-w-3xl">
            {sub}
          </p>
        </div>
      </div>
    </motion.section>
  );
}

export function StartPage() {
  const scopeRef = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      gsap.fromTo(
        ".dx-reveal",
        { opacity: 0, y: 22 },
        {
          opacity: 1,
          y: 0,
          duration: 0.55,
          stagger: 0.07,
          ease: "power2.out",
          scrollTrigger: {
            trigger: scopeRef.current,
            start: "top 80%",
          },
        },
      );

      gsap.fromTo(
        ".dx-bar",
        { scaleX: 0.08 },
        {
          scaleX: 1,
          duration: 0.9,
          ease: "power2.out",
          transformOrigin: "left center",
          stagger: 0.08,
          scrollTrigger: {
            trigger: ".dx-bench-wrap",
            start: "top 82%",
          },
        },
      );

      gsap.utils.toArray<HTMLElement>(".dx-counter").forEach((node) => {
        const target = Number(node.dataset.value ?? 0);
        const suffix = node.dataset.suffix ?? "";
        const counter = { value: 0 };

        gsap.to(counter, {
          value: target,
          duration: 1.2,
          ease: "power2.out",
          scrollTrigger: {
            trigger: node,
            start: "top 88%",
            once: true,
          },
          onUpdate: () => {
            node.textContent = `${Math.round(counter.value)}${suffix}`;
          },
          onComplete: () => {
            node.textContent = `${target}${suffix}`;
          },
        });
      });

      gsap.fromTo(
        ".dx-story-card",
        { opacity: 0, y: 20 },
        {
          opacity: 1,
          y: 0,
          duration: 0.55,
          stagger: 0.12,
          ease: "power2.out",
          scrollTrigger: {
            trigger: ".dx-story-rail",
            start: "top 82%",
            once: true,
          },
        },
      );

      gsap.fromTo(
        ".dx-deep-dive-card",
        { opacity: 0, y: 16 },
        {
          opacity: 1,
          y: 0,
          duration: 0.45,
          stagger: 0.08,
          ease: "power2.out",
          scrollTrigger: {
            trigger: ".dx-deep-dive-grid",
            start: "top 84%",
            once: true,
          },
        },
      );

      gsap.fromTo(
        ".dx-play-step",
        { opacity: 0, y: 16 },
        {
          opacity: 1,
          y: 0,
          duration: 0.45,
          stagger: 0.06,
          ease: "power2.out",
          scrollTrigger: {
            trigger: ".dx-playbook-grid",
            start: "top 84%",
            once: true,
          },
        },
      );

      gsap.fromTo(
        ".dx-video-card",
        { opacity: 0, y: 14 },
        {
          opacity: 1,
          y: 0,
          duration: 0.45,
          stagger: 0.08,
          ease: "power2.out",
          scrollTrigger: {
            trigger: ".dx-video-card",
            start: "top 90%",
            once: true,
          },
        },
      );
    },
    { scope: scopeRef },
  );

  return (
    <div ref={scopeRef} className="min-h-screen bg-background pb-20">
      <section className="pt-32 sm:pt-24">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <div className="dx-reveal text-center">
            {/* <p className="text-xs uppercase tracking-wide text-muted-foreground">Launching March 3, 2026</p> */}
            <div className="flex justify-center">
              <DxAiFace size={280} interactive={true} />
            </div>
            <p className="mt-6 text-base sm:text-lg text-muted-foreground">Hi. I&apos;m DX.</p>
            <h1 className="mt-3 font-serif text-4xl sm:text-5xl lg:text-6xl leading-tight text-foreground">
              The Developer Experience You Actually Deserve.
            </h1>
            <p className="mt-6 text-base sm:text-lg text-muted-foreground max-w-3xl mx-auto">
              DX is not just another AI app. It is a unified development
              experience platform that connects code, research, orchestration,
              and media execution in one runtime — built to help teams ship
              faster with less tool friction.
            </p>
            
            {/* Platform Download Cards */}
            <PlatformDownloadCards />
            
            <div className="mt-8 flex flex-col sm:flex-row justify-center gap-3">
              <Button asChild className="btn-inverse h-11 px-6">
                <Link href="/download">Get Started Free</Link>
              </Button>
              <Button asChild variant="outline" className="h-11 px-6">
                <a href="#showcases">Watch Demo</a>
              </Button>
            </div>
          </div>

          <motion.div
            className="mt-12 grid grid-cols-1 md:grid-cols-3 gap-4"
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true, amount: 0.2 }}
            transition={{ duration: 0.5, delay: 0.1 }}
          >
            {[
              "Built on Rust + GPUI",
              "100+ LLM providers + offline local models",
              "30–90% token savings on large operations",
            ].map((item) => (
              <Card key={item}>
                <CardContent className="p-4 text-sm text-foreground">
                  {item}
                </CardContent>
              </Card>
            ))}
          </motion.div>

          <motion.div
            className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4"
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.2 }}
            transition={{ duration: 0.4 }}
          >
            {stats.map((item) => (
              <Card key={item.label}>
                <CardContent className="p-4">
                  <p
                    className="text-2xl sm:text-3xl text-foreground dx-counter"
                    data-value={item.value}
                    data-suffix={item.suffix}
                  >
                    0{item.suffix}
                  </p>
                  <p className="mt-1 text-xs sm:text-sm text-muted-foreground">{item.label}</p>
                </CardContent>
              </Card>
            ))}
          </motion.div>
        </div>
      </section>

      {/* === FEATURE HERO BANNERS === */}
      <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
        <Separator className="my-8" />
        <motion.div
          className="text-center"
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, amount: 0.2 }}
          transition={{ duration: 0.5 }}
        >
          <h2 className="font-serif text-3xl sm:text-4xl text-foreground">
            Game-Changing Features That Set DX Apart
          </h2>
          <p className="mt-3 text-muted-foreground max-w-2xl mx-auto">
            Every feature is designed to go viral. This is not incremental — it is a paradigm shift.
          </p>
        </motion.div>
      </div>

      {heroFeatures.map((feature, index) => (
        <FeatureHeroBanner
          key={feature.label}
          label={feature.label}
          headline={feature.headline}
          sub={feature.sub}
          imgIndex={index}
        />
      ))}

      <div className="dx-home-section">
        <Section
          id="story-engine"
          title="The DX Story Engine"
          subtitle="A single runtime that compresses cost, keeps context, and scales from local execution to multi-provider production workflows."
        >
          <div className="dx-story-rail grid grid-cols-1 md:grid-cols-2 gap-4">
            {storyRail.map((item, idx) => (
              <motion.div
                key={item.step}
                className="dx-story-card border border-border p-5"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <p className="text-xs uppercase tracking-wide text-muted-foreground">
                  Step {item.step}
                </p>
                <h3 className="mt-2 text-lg text-foreground font-medium">{item.title}</h3>
                <p className="mt-2 text-sm text-foreground">{item.metric}</p>
                <p className="mt-3 text-sm text-muted-foreground">{item.detail}</p>
                <div className="relative mt-4 h-28 overflow-hidden border border-border">
                  <Image
                    src={thumb(idx + 15)}
                    alt={item.title}
                    fill
                    className="object-cover"
                    sizes="400px"
                    quality={95}
                  />
                  <div className="absolute inset-0 bg-black/30 flex items-center justify-center">
                    <span className="text-white text-2xl">▶</span>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>

          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              This is where DX changes the game: the same workflow can move from local, offline generation to cloud orchestration without rebuilding the process.
            </CardContent>
          </Card>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="what-is-dx"
          title="What Is DX?"
          subtitle="DX is a unified development experience platform built to serve everyone — developers, creators, and teams. There are no arbitrary category boundaries — everything is one connected system."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm sm:text-base">
            <Card>
              <CardContent className="p-5 text-muted-foreground">
              AI generation, tool calling, media creation, and workflow
              integration are not separate products. They are facets of one
              cohesive experience.
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5 text-muted-foreground">
              You can generate code, analyze data, run deep research, and
              produce media with one consistent workflow, one context, and one
              mental model.
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5 text-muted-foreground">
              Manage everything from the browser with a remote web console —
              then keep the same workflow on native apps across desktop, mobile,
              and companion devices.
              </CardContent>
            </Card>
          </div>

          <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Modes</p>
                <p className="mt-2 text-foreground">
                  Ask, Agent, Plan, Search, Study, Research.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Extensions</p>
                <p className="mt-2 text-foreground">
                  Browsers, IDEs, Figma, Photoshop, DaVinci Resolve — and more.
                </p>
              </CardContent>
            </Card>
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="deep-dive"
          title="Deep-Dive Routes"
          subtitle="Start on the high-impact homepage, then drill into full walkthroughs for each major DX capability surface."
        >
          <div className="dx-deep-dive-grid grid grid-cols-1 md:grid-cols-2 gap-4">
            {deepDiveRoutes.map((route) => (
              <motion.a
                key={route.title}
                href={route.href}
                className="dx-deep-dive-card border border-border p-5 block"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <h3 className="text-foreground text-lg font-medium">{route.title}</h3>
                <p className="mt-2 text-sm text-muted-foreground">{route.description}</p>
                <p className="mt-4 text-sm text-foreground">Open route →</p>
              </motion.a>
            ))}
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="command-center"
          title="DX Command Center"
          subtitle="Move from landing-page overview to operational proof in one click."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Product Workflows</p>
                <p className="mt-2 text-foreground">
                  Explore ask/agent/research execution patterns and connected context flows.
                </p>
                <div className="mt-4">
                  <Button asChild variant="outline" className="h-10 px-4">
                    <a href="/assistant">Open Assistant</a>
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Integration Surface</p>
                <p className="mt-2 text-foreground">
                  Validate MCP routing, provider coverage, and tool-call interfaces across clients.
                </p>
                <div className="mt-4 flex flex-wrap gap-2">
                  <Button asChild variant="outline" className="h-10 px-4">
                    <a href="/mcp">Open MCP</a>
                  </Button>
                  <Button asChild variant="outline" className="h-10 px-4">
                    <a href="/integrations">Open Integrations</a>
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Docs + API Readiness</p>
                <p className="mt-2 text-foreground">
                  Go deeper into setup, architecture notes, and workflow documentation.
                </p>
                <div className="mt-4">
                  <Button asChild variant="outline" className="h-10 px-4">
                    <a href="/docs">Open Docs</a>
                  </Button>
                </div>
              </CardContent>
            </Card>
          </div>

          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              DX is built for execution at scale: local-first workflows, cloud orchestration, and production-grade tool connectivity without context loss.
            </CardContent>
          </Card>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="built-on-rust"
          title="Built on Rust. Not Node.js. Not Electron."
          subtitle="DX is engineered in Rust for performance, efficiency, and native-grade responsiveness across platforms."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Speed</p>
                <p className="mt-2 text-foreground">
                  Near-native performance on every operation.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Efficiency</p>
                <p className="mt-2 text-foreground">
                  Designed to save RAM and stay responsive under heavy workloads.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Desktop UI</p>
                <p className="mt-2 text-foreground">
                  GPUI-powered rendering for a fast, responsive native experience.
                </p>
              </CardContent>
            </Card>
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="generate-anything"
          title="Generate Literally Anything"
          subtitle="If you can name it, DX can generate it."
        >
          <div className="overflow-x-auto border border-border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Category</TableHead>
                  <TableHead>Capabilities</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {generationCategories.map((row) => (
                  <TableRow key={row.category}>
                    <TableCell className="text-foreground">{row.category}</TableCell>
                    <TableCell className="text-muted-foreground">{row.capability}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="token-revolution"
          title="Token Revolution"
          subtitle="RLM + DX Serializer + tokenizers + micro-optimizations across the full pipeline."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-foreground">RLM</p>
                <p className="mt-2 text-muted-foreground text-sm">
                  Saves 80–90% tokens on large file operations by minimizing
                  reference length in context flows.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-foreground">DX Serializer</p>
                <p className="mt-2 text-muted-foreground text-sm">
                  Saves 70–90% tokens on tool calls by replacing bloated JSON
                  transport.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-foreground">Tokenizers</p>
                <p className="mt-2 text-muted-foreground text-sm">
                  Image tokenization + compact encodings reduce payload size and
                  keep multimodal workflows affordable.
                </p>
              </CardContent>
            </Card>
          </div>

          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              Token savings compound across the entire workflow, which makes
              deeper multi-step agents viable — online or offline.
            </CardContent>
          </Card>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="works-everywhere"
          title="Works Everywhere"
          subtitle="Native apps and extensions across the full development and creative workflow."
        >
          <div className="overflow-x-auto border border-border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Platform</TableHead>
                  <TableHead>App Type</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {platforms.map(([platform, appType, status]) => (
                  <TableRow key={platform}>
                    <TableCell className="text-foreground">{platform}</TableCell>
                    <TableCell className="text-muted-foreground">{appType}</TableCell>
                    <TableCell className="text-foreground">{status}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="free-ai"
          title="Free AI Access — Any Provider, Even Offline"
          subtitle="Own your workflow. No vendor lock-in."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card>
              <CardContent className="p-5 text-muted-foreground">
                <p className="text-foreground">Online</p>
                <p className="mt-2 text-sm">
                  Connect to 100+ LLM providers, open-source models, and
                  self-hosted endpoints.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5 text-muted-foreground">
                <p className="text-foreground">Offline</p>
                <p className="mt-2 text-sm">
                  Run capable local models without internet and without token
                  limits.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5 text-muted-foreground">
                <p className="text-foreground">Hybrid</p>
                <p className="mt-2 text-sm">
                  Switch seamlessly between cloud and local models based on
                  runtime conditions.
                </p>
              </CardContent>
            </Card>
          </div>

          <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Integrations</p>
                <p className="mt-2 text-foreground">
                  400+ connects. Link Cloud CLI skills, plugins, and communication
                  apps like WhatsApp, Telegram, and Discord.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Forge</p>
                <p className="mt-2 text-foreground">
                  Version control for code and viral-ready media — with
                  bring-your-own storage connectors.
                </p>
              </CardContent>
            </Card>
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="competitive"
          title="Competitive Positioning"
          subtitle="Technical differences that matter in production workflows."
        >
          <div className="overflow-x-auto border border-border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Feature</TableHead>
                  <TableHead>DX</TableHead>
                  <TableHead>Competitors</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {comparisons.map(([feature, dx, competitors]) => (
                  <TableRow key={feature}>
                    <TableCell className="text-foreground">{feature}</TableCell>
                    <TableCell className="text-foreground">{dx}</TableCell>
                    <TableCell className="text-muted-foreground">{competitors}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          <div className="mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">
                  Forge Storage Strategy
                </p>
                <p className="mt-2 text-foreground">
                  Store media to your own platforms: YouTube (unlisted/draft) for
                  video, Pinterest for images, SoundCloud/Spotify-like for audio,
                  Sketchfab-like for 3D, and Git providers for code/docs.
                </p>
                <p className="mt-3 text-sm text-muted-foreground">
                  Also supports R2 buckets (pro fallback) and common cloud drives.
                  Availability depends on provider policies.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Traffic Security</p>
                <p className="mt-2 text-foreground">
                  Green / Yellow / Red safety levels. DX auto-executes harmless
                  work, warns on sensitive actions, and adds backups on risky
                  operations.
                </p>
                <p className="mt-3 text-sm text-muted-foreground">
                  Sensitive data is hashed/redacted before third-party calls;
                  optional firewall/VPN/IDS-style protections are part of the DX
                  security layer.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Check Score</p>
                <p className="mt-2 text-foreground">
                  Anime-style 500 score rank system (F → SSSSS) based on naming,
                  structure, and issues — with suggestions to improve security and
                  quality.
                </p>
                <p className="mt-3 text-sm text-muted-foreground">
                  Includes vulnerability scanning plus code/media linting and
                  audit reports.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Media + Workspace</p>
                <p className="mt-2 text-foreground">
                  Fetch, organize, and version media across platforms — plus a
                  clean workspace across IDEs, token-efficient serialization, and
                  built-in i18n (translate + STT/TTS).
                </p>
                <p className="mt-3 text-sm text-muted-foreground">
                  Driven workflows (spec-first) + DCP for faster, more
                  token-efficient agent execution.
                </p>
              </CardContent>
            </Card>
          </div>
        </Section>
      </div>

      <FeatureHeroBanner
        label="DX Forge"
        headline="Viral-Ready Version Control — Code, Media, Models, Everything"
        sub="Store output to your own channels, version every asset, and distribute with a built-in flywheel. This is version control for creators and developers alike."
        imgIndex={6}
      />

      <div className="dx-home-section">
        <Section
          id="forge"
          title="Forge: Viral-Ready Version Control for More Than Code"
          subtitle="DX Forge routes output to user-owned platforms so teams get resilient storage + distribution-ready workflows."
        >
          <div className="overflow-x-auto border border-border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Asset Type</TableHead>
                  <TableHead>Primary Storage Route</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {forgeRoutes.map(([type, route]) => (
                  <TableRow key={type}>
                    <TableCell className="text-foreground">{type}</TableCell>
                    <TableCell className="text-muted-foreground">{route}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              Pro fallback storage remains available (R2 + cloud drives like Google Drive/Dropbox) while default routes prioritize user-owned channels.
            </CardContent>
          </Card>

          <div className="dx-playbook-grid mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {pillarPlaybooks.forge.map((step, index) => (
              <motion.div
                key={step}
                className="dx-play-step border border-border p-4"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Forge Step {index + 1}
                </p>
                <p className="mt-2 text-sm text-foreground">{step}</p>
              </motion.div>
            ))}
          </div>

          <VideoPlaceholderStrip
            title="Forge Demo Reel"
            items={pillarVideos.forge}
            startIndex={10}
          />
        </Section>
      </div>

      <FeatureHeroBanner
        label="Traffic Security"
        headline="Your AI Agent Stays Autonomous — And Safe"
        sub="Green means go. Yellow means warn. Red means guard. DX moves fast without sacrificing trust, and your sensitive data never reaches third-party calls unprotected."
        imgIndex={7}
      />

      <div className="dx-home-section">
        <Section
          id="traffic-security"
          title="Traffic Security: Green · Yellow · Red"
          subtitle="DX agents stay autonomous while preserving user safety with level-based execution behavior."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {trafficLevels.map((item) => (
              <Card key={item.level}>
                <CardContent className="p-5">
                  <p className="text-sm text-muted-foreground">{item.level}</p>
                  <p className="mt-2 text-foreground">{item.behavior}</p>
                  <p className="mt-3 text-sm text-muted-foreground">{item.detail}</p>
                </CardContent>
              </Card>
            ))}
          </div>

          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              Sensitive fields are protected before third-party calls, and high-risk actions receive additional safeguards and monitoring.
            </CardContent>
          </Card>

          <div className="dx-playbook-grid mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {pillarPlaybooks.traffic.map((step, index) => (
              <motion.div
                key={step}
                className="dx-play-step border border-border p-4"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Security Step {index + 1}
                </p>
                <p className="mt-2 text-sm text-foreground">{step}</p>
              </motion.div>
            ))}
          </div>

          <VideoPlaceholderStrip
            title="Traffic Security Demo Reel"
            items={pillarVideos.traffic}
            startIndex={14}
          />
        </Section>
      </div>

      <FeatureHeroBanner
        label="DX Check"
        headline="500-Point Code + Media Quality Score That Actually Helps You Improve"
        sub="From F to SSSSS — an anime-inspired rank system that audits naming, structure, security, and quality. Get a ranked score and fix guidance in seconds."
        imgIndex={8}
      />

      <div className="dx-home-section">
        <Section
          id="check"
          title="Check: 500-Point Rank System"
          subtitle="DX Check audits naming, structure, quality, and security to produce a ranked score with direct fix guidance."
        >
          <Card>
            <CardContent className="p-5">
              <p className="text-sm text-muted-foreground">Rank Ladder</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {checkRanks.map((rank) => (
                  <Badge key={rank} variant="outline">
                    {rank}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>

          <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
            <Card><CardContent className="p-5 text-sm text-muted-foreground">Security and vulnerability scanning</CardContent></Card>
            <Card><CardContent className="p-5 text-sm text-muted-foreground">Code + media linting for quality consistency</CardContent></Card>
            <Card><CardContent className="p-5 text-sm text-muted-foreground">Actionable report with prioritized improvements</CardContent></Card>
          </div>

          <div className="dx-playbook-grid mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {pillarPlaybooks.check.map((step, index) => (
              <motion.div
                key={step}
                className="dx-play-step border border-border p-4"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Check Step {index + 1}
                </p>
                <p className="mt-2 text-sm text-foreground">{step}</p>
              </motion.div>
            ))}
          </div>

          <VideoPlaceholderStrip
            title="Check Demo Reel"
            items={pillarVideos.check}
            startIndex={18}
          />
        </Section>
      </div>

      <FeatureHeroBanner
        label="Media Engine"
        headline="Multimodal Generation, Workspace Discipline, and Token-Efficient Transport"
        sub="Generate images, video, audio, and 3D. Organize with Driven spec-first workflows. Ship faster with DX Serializer's 70–90% token savings on every tool call."
        imgIndex={9}
      />

      <div className="dx-home-section">
        <Section
          id="media-workspace"
          title="Media, Workspace, Serializer, i18n, Driven, DCP"
          subtitle="DX combines multimodal creation with a disciplined workspace and transport layer built for practical production speed."
        >
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {mediaPillars.map((pillar) => (
              <Card key={pillar}>
                <CardContent className="p-5 text-sm text-muted-foreground">
                  {pillar}
                </CardContent>
              </Card>
            ))}
          </div>

          <div className="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Workspace + Driven</p>
                <p className="mt-2 text-foreground">
                  Keep execution spec-driven instead of chaotic vibe-coding flows, with cleaner context across IDEs and tools.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm text-muted-foreground">Serializer + i18n + DCP</p>
                <p className="mt-2 text-foreground">
                  Human-readable, token-efficient transport with global language workflows and faster MCP/ACP/A2A-compatible execution.
                </p>
              </CardContent>
            </Card>
          </div>

          <div className="dx-playbook-grid mt-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {pillarPlaybooks.media.map((step, index) => (
              <motion.div
                key={step}
                className="dx-play-step border border-border p-4"
                whileHover={{ y: -2 }}
                transition={{ duration: 0.2 }}
              >
                <p className="text-xs text-muted-foreground uppercase tracking-wide">
                  Media Step {index + 1}
                </p>
                <p className="mt-2 text-sm text-foreground">{step}</p>
              </motion.div>
            ))}
          </div>

          <VideoPlaceholderStrip
            title="Media Workflow Demo Reel"
            items={pillarVideos.media}
            startIndex={1}
          />
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="built-in-tools"
          title="Built-In Tools That Transform How You Work"
          subtitle="Every DX workflow ships with a complete power suite — no plugins required."
        >
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {builtInTools.map((tool) => (
              <Card key={tool.name}>
                <CardContent className="p-5">
                  <p className="text-foreground font-medium">{tool.name}</p>
                  <p className="mt-2 text-sm text-muted-foreground">{tool.desc}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="extensions"
          title="Extensions: DX Everywhere You Already Work"
          subtitle="One context. Every tool. Browsers, IDEs, design, video, and communication — all connected."
        >
          <div className="overflow-x-auto border border-border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Category</TableHead>
                  <TableHead>Supported Tools</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {extensionsList.map((ext) => (
                  <TableRow key={ext.category}>
                    <TableCell className="text-foreground font-medium">{ext.category}</TableCell>
                    <TableCell className="text-muted-foreground">{ext.items}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <Card className="mt-6">
            <CardContent className="p-5 text-sm text-muted-foreground">
              Every extension shares the same DX context, model access, and token-saving pipeline — so you never lose state when you switch tools.
            </CardContent>
          </Card>
        </Section>
      </div>

      <div className="dx-home-section">
        <Section
          id="pricing"
          title="Pricing"
          subtitle="Free forever. No credit card required."
        >
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {[
              {
                title: "Free",
                price: "Free",
                details:
                  "All features included. Unlimited usage. No hidden costs.",
              },
              {
                title: "Forever",
                price: "Free",
                details:
                  "Core generation, local workflows, integrations, and more.",
              },
              {
                title: "Open Source",
                price: "Free",
                details:
                  "Built by developers, for developers. Always free.",
              },
            ].map((plan) => (
              <Card
                key={plan.title}
                className={plan.title === "Forever" ? "ring-2 ring-primary" : ""}
              >
                <CardContent className="p-5">
                  <p className="text-sm text-muted-foreground">{plan.title}</p>
                  <p className="mt-2 text-2xl text-foreground">{plan.price}</p>
                  <p className="mt-3 text-sm text-muted-foreground">
                    {plan.details}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        </Section>
      </div>

      <section className="dx-reveal pt-14">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <Card className="dx-bench-wrap">
            <CardContent className="p-5 sm:p-7">
            <h2 className="font-serif text-3xl text-foreground">Built With Rust. Built To Fly.</h2>
            <p className="mt-3 text-muted-foreground max-w-3xl">
              DX is engineered in Rust for memory safety and high throughput. It keeps startup instant, UI responsive,
              and workflows stable under heavy project loads.
            </p>

            <div className="mt-8 grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="space-y-4">
                {[
                  ["DX", "12ms startup"],
                  ["VS Code", "1.2s startup"],
                  ["JetBrains", "3.5s startup"],
                ].map(([label, value]) => (
                  <div key={label}>
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-foreground">{label}</span>
                      <span className="text-muted-foreground">{value}</span>
                    </div>
                    <div className="mt-2 h-2 bg-secondary/40 border border-border overflow-hidden">
                      <div className="dx-bar h-full bg-foreground" />
                    </div>
                  </div>
                ))}
              </div>

              <Card>
                <CardContent className="p-4">
                <p className="text-sm text-muted-foreground">Deep Dive</p>
                <p className="mt-2 text-foreground">
                  Zero garbage collection pauses. Low memory pressure. Native execution path for editor and assistant workflows.
                </p>
                <p className="mt-4 text-sm text-muted-foreground">
                  12ms startup · 45MB RAM baseline · 60fps UI under heavy file and tool workloads.
                </p>
              </CardContent>
            </Card>
            </div>
            </CardContent>
          </Card>
        </div>
      </section>

      <section className="dx-reveal pt-14">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <Card>
            <CardContent className="p-5 sm:p-7">
              <h2 className="font-serif text-3xl text-foreground">Developer Testimonials</h2>
              <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-4">
                {testimonials.map((item) => (
                  <Card key={item.quote}>
                    <CardContent className="p-4">
                      <p className="text-foreground">“{item.quote}”</p>
                      <p className="mt-3 text-xs text-muted-foreground">— {item.by}</p>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      </section>

      <section className="dx-reveal pt-14">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <Card className="overflow-x-auto">
            <CardContent className="p-5 sm:p-7">
            <h2 className="font-serif text-3xl text-foreground">Comparison Table</h2>
            <Table className="mt-5 min-w-[680px]">
              <TableHeader>
                <TableRow>
                  <TableHead>Feature</TableHead>
                  <TableHead>DX</TableHead>
                  <TableHead>VS Code</TableHead>
                  <TableHead>JetBrains</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {comparisonRows.map((row) => (
                  <TableRow key={row[0]}>
                    <TableCell className="text-foreground">{row[0]}</TableCell>
                    <TableCell className="text-foreground">{row[1]}</TableCell>
                    <TableCell className="text-muted-foreground">{row[2]}</TableCell>
                    <TableCell className="text-muted-foreground">{row[3]}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* <section className="dx-reveal pt-14">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <Card className="text-center">
            <CardContent className="p-6 sm:p-10">
            <p className="text-xs uppercase tracking-wide text-muted-foreground">Interactive Demo</p>
            <h3 className="mt-3 font-serif text-3xl sm:text-4xl text-foreground">Try the DX workflow playground.</h3>
            <p className="mt-4 text-muted-foreground max-w-2xl mx-auto">
              See connected generation, MCP actions, automations, and offline mode behavior in one guided demo.
            </p>
            <div className="mt-7 flex flex-wrap justify-center gap-3">
              <Button asChild className="btn-inverse h-11 px-8">
                <Link href="/assistant">Open Assistant</Link>
              </Button>
              <Button asChild variant="outline" className="h-11 px-8">
                <Link href="/integrations">Explore Integrations</Link>
              </Button>
            </div>
            </CardContent>
          </Card>
        </div>
      </section> */}

      <section id="waitlist" className="dx-reveal pt-14">
        <div className="max-w-[1150px] mx-auto px-4 sm:px-8">
          <motion.div
            className="border border-border p-6 sm:p-10 text-center"
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.2 }}
            transition={{ duration: 0.35 }}
          >
            <p className="text-muted-foreground text-sm uppercase tracking-wide">
              Early Access
            </p>
            <h3 className="mt-3 font-serif text-3xl sm:text-4xl text-foreground">
              Be first on DX launch day.
            </h3>
            <p className="mt-4 text-muted-foreground max-w-2xl mx-auto">
              Launching March 3, 2026. Join the waitlist for priority
              access, release notes, and first-week benchmarks.
            </p>
            <div className="mt-7 flex justify-center">
              <Button asChild className="btn-inverse h-11 px-8">
                <a href="mailto:hello@dx.ai?subject=DX%20Early%20Access">
                  Join Waitlist
                </a>
              </Button>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  );
}
