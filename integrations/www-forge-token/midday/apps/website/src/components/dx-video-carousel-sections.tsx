"use client";

import { useGSAP } from "@gsap/react";
import { Button } from "@midday/ui/button";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { motion } from "motion/react";
import { useRef } from "react";

gsap.registerPlugin(ScrollTrigger);

type DxSection = {
  id: string;
  title: string;
  description: string;
  bullets: string[];
};

const dxSections: DxSection[] = [
  {
    id: "token-connection",
    title: "Token Saving → Connection Engine",
    description:
      "In DX, token efficiency and workflow connection are the same system. RLM + DX Serializer + tokenizers reduce waste while keeping every tool action connected in one live context.",
    bullets: [
      "RLM saves 80–90% tokens on large file operations.",
      "DX Serializer saves 70–90% tokens on tool calls.",
      "Compound savings enable deeper multi-step workflows at lower cost.",
    ],
  },
  {
    id: "3d-speed",
    title: "3D Speed, Rust Core",
    description:
      "DX is built in Rust, so heavy generation (including 3D/AR/VR) runs faster with lower memory overhead and better sustained performance.",
    bullets: [
      "More throughput with less RAM pressure.",
      "Low-end and high-end hardware both stay responsive.",
      "Native pipeline avoids Electron bottlenecks.",
    ],
  },
  {
    id: "offline-capability",
    title: "Offline Capability",
    description:
      "DX stays productive offline with local model execution and no token limits, then synchronizes cloud workflows when available.",
    bullets: [
      "Run local generation without internet.",
      "No offline token ceilings.",
      "Seamless cloud fallback when online.",
    ],
  },
  {
    id: "connects-400",
    title: "400+ Connects, One Runtime",
    description:
      "DX connects providers, tools, and apps (including communication platforms) so model orchestration, tool calls, and media pipelines stay unified in one interface.",
    bullets: [
      "400+ connects with shared context.",
      "Link Cloud CLI skills, plugins, and workflows.",
      "Connect WhatsApp, Telegram, Discord, and more.",
    ],
  },
  {
    id: "modes-extensions",
    title: "Modes + Extensions Everywhere",
    description:
      "DX is outcome-first: switch between modes and keep the same workflow across native apps, browser, and production software extensions.",
    bullets: [
      "Modes: Ask, Agent, Plan, Search, Study, Research.",
      "Extensions for browsers, IDEs, Figma, Photoshop, DaVinci Resolve.",
      "Remote web console to manage all devices from the browser.",
    ],
  },
  {
    id: "forge-media-vcs",
    title: "Forge: Version Control for Media",
    description:
      "Forge brings version control to more than code: videos, images, audio, documents, and 3D assets — with bring-your-own storage connectors.",
    bullets: [
      "Video → store to your YouTube channel as unlisted/draft.",
      "Images → store to your Pinterest account.",
      "Code/docs → store to GitHub/GitLab/Bitbucket (one or all).",
    ],
  },
  {
    id: "traffic-security",
    title: "Traffic Security",
    description:
      "DX agents automate work without ignoring risk: a simple Green / Yellow / Red system controls execution, warnings, and safeguards.",
    bullets: [
      "Green: auto-executes harmless tasks.",
      "Yellow: proceeds with warnings/notifications.",
      "Red: adds stronger warnings and creates safety backups on destructive actions.",
    ],
  },
  {
    id: "check-score",
    title: "Check: 500 Score Rank System",
    description:
      "DX Check audits structure, naming, and issues to produce a 500 score with anime-style ranks (F → SSSSS), plus actionable improvement suggestions.",
    bullets: [
      "Security audit + vulnerability scanning.",
      "Code linting and media linting.",
      "Detailed reports with fix suggestions.",
    ],
  },
  {
    id: "media-platforms",
    title: "Media: Platforms, Library, Editing",
    description:
      "DX supports modern media workflows across audio, video, images, and 3D/AR/VR — with provider connectors, a library, and practical editing tools.",
    bullets: [
      "Fetch from providers (images/video/audio/3D) and your linked accounts.",
      "Organize assets in a media library and collaborate.",
      "Fonts + icon sets + versioning for media assets.",
    ],
  },
  {
    id: "workspace-driven-dcp",
    title: "Workspace + Serializer + i18n + Driven + DCP",
    description:
      "DX keeps workspaces clean across IDEs, makes tool calls more token-efficient, and stays spec-driven so agent workflows stay reliable instead of chaotic.",
    bullets: [
      "Workspace hygiene across IDEs and tools.",
      "DX Serializer for fast, readable, compact transport.",
      "Built-in i18n: translate + STT/TTS workflows.",
    ],
  },
];

function VideoCarousel({ sectionId }: { sectionId: string }) {
  const trackRef = useRef<HTMLDivElement>(null);

  const scrollByViewport = (direction: "next" | "prev") => {
    const track = trackRef.current;
    if (!track) return;
    const amount = track.clientWidth * 0.9;
    track.scrollBy({
      left: direction === "next" ? amount : -amount,
      behavior: "smooth",
    });
  };

  return (
    <div className="mt-6">
      <div className="flex items-center justify-between mb-3">
        <p className="text-xs uppercase tracking-wide text-muted-foreground">
          Demo carousel
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
        className="dx-video-track flex gap-4 overflow-x-auto snap-x snap-mandatory pb-2"
        data-section={sectionId}
      >
        {Array.from({ length: 4 }).map((_, index) => (
          <div
            key={`${sectionId}-video-${index + 1}`}
            className="dx-video-card snap-start shrink-0 w-[92%] md:w-[52%] lg:w-[40%] border border-border bg-secondary/20"
          >
            <div className="aspect-video border-b border-border flex items-center justify-center text-sm text-muted-foreground">
              Video slot {index + 1}
            </div>
            <div className="p-4">
              <p className="text-sm text-foreground">DX Demo Placeholder</p>
              <p className="text-xs text-muted-foreground mt-1">
                Replace with your actual product video for {sectionId}.
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function DxVideoCarouselSections({
  pageTitle,
  pageDescription,
}: {
  pageTitle: string;
  pageDescription: string;
}) {
  const scopeRef = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      gsap.fromTo(
        ".dx-fade-in",
        { opacity: 0, y: 28 },
        {
          opacity: 1,
          y: 0,
          duration: 0.55,
          ease: "power2.out",
          stagger: 0.08,
          scrollTrigger: {
            trigger: scopeRef.current,
            start: "top 78%",
          },
        },
      );

      gsap.utils.toArray<HTMLElement>(".dx-video-card").forEach((card) => {
        gsap.fromTo(
          card,
          { opacity: 0.45, y: 20 },
          {
            opacity: 1,
            y: 0,
            duration: 0.5,
            ease: "power2.out",
            scrollTrigger: {
              trigger: card,
              start: "top 88%",
            },
          },
        );
      });
    },
    { scope: scopeRef },
  );

  return (
    <div ref={scopeRef} className="min-h-[calc(100vh-180px)] pb-24">
      <section className="pt-32 sm:pt-36 pb-10 sm:pb-14">
        <div className="max-w-[1100px] mx-auto px-4 sm:px-8">
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
            className="dx-fade-in"
          >
            <p className="text-xs uppercase tracking-wide text-muted-foreground">
              DX Route
            </p>
            <h1 className="mt-3 font-serif text-3xl sm:text-4xl lg:text-5xl text-foreground">
              {pageTitle}
            </h1>
            <p className="mt-4 text-base text-muted-foreground max-w-3xl">
              {pageDescription}
            </p>
          </motion.div>
        </div>
      </section>

      <div className="max-w-[1100px] mx-auto px-4 sm:px-8 space-y-10">
        {dxSections.map((section) => (
          <section
            key={section.id}
            className="dx-fade-in border border-border p-5 sm:p-7"
          >
            <h2 className="font-serif text-2xl text-foreground">
              {section.title}
            </h2>
            <p className="mt-3 text-muted-foreground">{section.description}</p>
            <ul className="mt-4 space-y-2">
              {section.bullets.map((bullet) => (
                <li key={bullet} className="text-sm text-muted-foreground">
                  • {bullet}
                </li>
              ))}
            </ul>
            <VideoCarousel sectionId={section.id} />
          </section>
        ))}
      </div>
    </div>
  );
}
