"use client";

import { Button } from "@midday/ui/button";
import { motion } from "motion/react";
import Image from "next/image";
import { useMemo, useState } from "react";

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

type VideoItem = {
  title: string;
  subtitle: string;
};

function VideoPlaceholder({ item, index = 0 }: { item: VideoItem; index?: number }) {
  return (
    <div className="border border-border bg-background overflow-hidden">
      <div className="relative aspect-video border-b border-border">
        <Image
          src={thumb(index)}
          alt={item.title}
          fill
          className="object-cover"
          sizes="(max-width: 768px) 100vw, 600px"
        />
        <div className="absolute inset-0 bg-black/40 flex items-center justify-center">
          <div className="w-14 h-14 rounded-full border-2 border-white/80 flex items-center justify-center">
            <span className="text-white text-xl ml-0.5">▶</span>
          </div>
        </div>
      </div>
      <div className="p-4">
        <p className="text-sm text-foreground">{item.title}</p>
        <p className="text-xs text-muted-foreground mt-1">{item.subtitle} · Coming soon</p>
      </div>
    </div>
  );
}

function DotNav({ count, active, onChange }: { count: number; active: number; onChange: (index: number) => void }) {
  return (
    <div className="flex items-center justify-center gap-2 mt-4">
      {Array.from({ length: count }).map((_, index) => (
        <button
          key={`dot-${index}`}
          type="button"
          onClick={() => onChange(index)}
          aria-label={`Go to slide ${index + 1}`}
          className={`h-2 rounded-full transition-all ${active === index ? "w-8 bg-foreground" : "w-2 bg-muted-foreground/50"}`}
        />
      ))}
    </div>
  );
}

function ThreeUpCarousel() {
  const items = useMemo<VideoItem[]>(
    () => [
      { title: "RLM Compression Pipeline", subtitle: "Token Saving to Connection" },
      { title: "Context Retention Under Load", subtitle: "Token Saving to Connection" },
      { title: "Serializer Efficiency", subtitle: "Token Saving to Connection" },
      { title: "Compound Savings", subtitle: "Token Saving to Connection" },
    ],
    [],
  );
  const [active, setActive] = useState(1);

  const prev = () => setActive((current) => (current - 1 + items.length) % items.length);
  const next = () => setActive((current) => (current + 1) % items.length);

  return (
    <div>
      <div className="flex justify-end gap-2 mb-3">
        <Button type="button" variant="outline" className="h-8 px-3" onClick={prev}>Prev</Button>
        <Button type="button" variant="outline" className="h-8 px-3" onClick={next}>Next</Button>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {[-1, 0, 1].map((offset) => {
          const index = (active + offset + items.length) % items.length;
          const isCenter = offset === 0;
          return (
            <motion.div
              key={`three-${index}-${offset}`}
              initial={{ opacity: 0.5, scale: 0.95 }}
              animate={{ opacity: isCenter ? 1 : 0.62, scale: isCenter ? 1 : 0.95 }}
              transition={{ duration: 0.35 }}
            >
              <VideoPlaceholder item={items[index] ?? items[0]!} index={index} />
            </motion.div>
          );
        })}
      </div>
      <DotNav count={items.length} active={active} onChange={setActive} />
    </div>
  );
}

function CenteredThumbCarousel() {
  const items = useMemo<VideoItem[]>(
    () => [
      { title: "Cross-Tool Graph", subtitle: "Everything Connected" },
      { title: "Editor + Terminal + AI", subtitle: "Everything Connected" },
      { title: "MCP Data Flow", subtitle: "Everything Connected" },
      { title: "Unified Runtime", subtitle: "Everything Connected" },
    ],
    [],
  );
  const [active, setActive] = useState(0);

  return (
    <div>
      <div className="max-w-4xl mx-auto">
        <VideoPlaceholder item={items[active] ?? items[0]!} index={active + 4} />
        {items.map((item, index) => (
          <button
            key={`thumb-${item.title}`}
            type="button"
            className={`border p-2 text-left transition-colors ${active === index ? "border-foreground" : "border-border hover:border-foreground/50"}`}
            onClick={() => setActive(index)}
          >
            <p className="text-xs text-foreground line-clamp-1">{item.title}</p>
            <p className="text-[11px] text-muted-foreground mt-1">{item.subtitle}</p>
          </button>
        ))}
      </div>
    </div>
  );
}

function FullBleedCaptionCarousel() {
  const items = useMemo<VideoItem[]>(
    () => [
      { title: "Instant File Open", subtitle: "Rust Speed" },
      { title: "Ultra-Low RAM Runtime", subtitle: "Rust Speed" },
      { title: "60fps UI Consistency", subtitle: "Rust Speed" },
      { title: "Massive Project Search", subtitle: "Rust Speed" },
      { title: "Parallel Build Assist", subtitle: "Rust Speed" },
    ],
    [],
  );
  const [active, setActive] = useState(0);

  return (
    <div>
      <VideoPlaceholder item={items[active] ?? items[0]!} index={active + 8} />
      <p className="mt-3 text-sm text-muted-foreground">Caption: {items[active]?.title}</p>
      <div className="mt-4 grid grid-cols-2 sm:grid-cols-5 gap-2">
        {items.map((item, index) => (
          <button
            key={`speed-${item.title}`}
            type="button"
            onClick={() => setActive(index)}
            className={`text-xs border px-2 py-2 text-left ${active === index ? "border-foreground text-foreground" : "border-border text-muted-foreground hover:text-foreground"}`}
          >
            {item.title}
          </button>
        ))}
      </div>
    </div>
  );
}

function SplitMcpCarousel() {
  const steps = useMemo(
    () => [
      {
        title: "Browse MCP App Store",
        subtitle: "Discover MCP apps for GitHub, Slack, databases, and more.",
      },
      {
        title: "One-Click Install",
        subtitle: "Add an MCP app to your DX workspace instantly.",
      },
      {
        title: "AI Gains Context",
        subtitle: "Assistant can now access the app's data and actions.",
      },
      {
        title: "Use in Workflow",
        subtitle: "Trigger MCP actions from commands, shortcuts, or chat.",
      },
    ],
    [],
  );
  const [active, setActive] = useState(0);

  return (
    <div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 border border-border p-4">
        <VideoPlaceholder item={{ title: steps[active]?.title ?? "MCP", subtitle: "MCP Apps Integration" }} index={active + 13} />
        <div className="border border-border p-4">
          <p className="text-sm text-muted-foreground">Step {active + 1} of {steps.length}</p>
          <h4 className="mt-2 text-xl font-serif text-foreground">{steps[active]?.title}</h4>
          <p className="mt-2 text-sm text-muted-foreground">{steps[active]?.subtitle}</p>
          <div className="mt-5 flex flex-wrap gap-2">
            {[
              "GitHub MCP",
              "Slack MCP",
              "PostgreSQL MCP",
              "Figma MCP",
              "Custom MCP",
            ].map((app) => (
              <span key={app} className="text-xs border border-border px-2 py-1 text-muted-foreground">{app}</span>
            ))}
          </div>
          <div className="mt-6 flex items-center gap-2">
            <Button asChild variant="outline" className="h-9 px-4"><a href="/integrations">Browse MCP Apps</a></Button>
            <Button asChild className="h-9 px-4 btn-inverse"><a href="/docs/mcp-apps">Build Custom MCP App</a></Button>
          </div>
        </div>
      </div>
      <DotNav count={steps.length} active={active} onChange={setActive} />
    </div>
  );
}

function CardStackOfflineCarousel() {
  const items = useMemo<VideoItem[]>(
    () => [
      { title: "Offline Editing", subtitle: "Offline Capability" },
      { title: "Local AI Runtime", subtitle: "Offline Capability" },
      { title: "Cached Docs", subtitle: "Offline Capability" },
      { title: "Sync on Reconnect", subtitle: "Offline Capability" },
    ],
    [],
  );

  return (
    <div className="overflow-x-auto pb-2">
      <div className="flex gap-4 min-w-max">
        {items.map((item, index) => (
          <motion.div
            key={`offline-${item.title}`}
            initial={{ opacity: 0.65, y: 10 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, amount: 0.2 }}
            transition={{ duration: 0.35, delay: index * 0.05 }}
            className="w-[290px] sm:w-[360px]"
          >
            <VideoPlaceholder item={item} index={index + 17} />
          </motion.div>
        ))}
      </div>
    </div>
  );
}

function TabbedWorkflowCarousel() {
  const tabs: Array<{ key: string; label: string; items: VideoItem[] }> = [
    {
      key: "shortcuts",
      label: "Shortcuts",
      items: [
        { title: "Command Palette", subtitle: "Shortcuts" },
        { title: "Quick Navigation", subtitle: "Shortcuts" },
        { title: "Multi-cursor Flow", subtitle: "Shortcuts" },
      ],
    },
    {
      key: "automations",
      label: "Automations",
      items: [
        { title: "Auto-Refactor Chain", subtitle: "Automations" },
        { title: "On-Save Actions", subtitle: "Automations" },
        { title: "Prompt Macros", subtitle: "Automations" },
      ],
    },
    {
      key: "workflows",
      label: "Workflows",
      items: [
        { title: "Bugfix Workflow", subtitle: "Workflows" },
        { title: "Release Workflow", subtitle: "Workflows" },
        { title: "Research Workflow", subtitle: "Workflows" },
      ],
    },
  ];

  const [activeTab, setActiveTab] = useState(tabs[0]?.key ?? "shortcuts");
  const [activeVideoIndex, setActiveVideoIndex] = useState(0);

  const tab = tabs.find((item) => item.key === activeTab) ?? tabs[0]!;
  const item = tab.items[activeVideoIndex] ?? tab.items[0]!;

  return (
    <div>
      <div className="grid grid-cols-3 gap-2 border border-border p-2 mb-4">
        {tabs.map((itemTab) => (
          <button
            key={itemTab.key}
            type="button"
            className={`h-9 text-sm border ${activeTab === itemTab.key ? "border-foreground text-foreground" : "border-border text-muted-foreground"}`}
            onClick={() => {
              setActiveTab(itemTab.key);
              setActiveVideoIndex(0);
            }}
          >
            {itemTab.label}
          </button>
        ))}
      </div>

      <VideoPlaceholder item={item} index={tabs.findIndex((t) => t.key === activeTab) * 3 + activeVideoIndex} />
      <div className="mt-4 flex flex-wrap gap-2">
        {tab.items.map((tabItem, index) => (
          <button
            key={`${tab.key}-${tabItem.title}`}
            type="button"
            onClick={() => setActiveVideoIndex(index)}
            className={`text-xs border px-2 py-1 ${activeVideoIndex === index ? "border-foreground text-foreground" : "border-border text-muted-foreground"}`}
          >
            {tabItem.title}
          </button>
        ))}
      </div>
    </div>
  );
}

function ShowcaseBlock({ title, subtitle, children }: { title: string; subtitle: string; children: React.ReactNode }) {
  return (
    <section className="border border-border p-4 sm:p-6 bg-background">
      <h3 className="font-serif text-2xl sm:text-3xl text-foreground">{title}</h3>
      <p className="mt-2 text-sm sm:text-base text-muted-foreground max-w-3xl">{subtitle}</p>
      <div className="mt-6">{children}</div>
    </section>
  );
}

export function DxVideoShowcases() {
  return (
    <div className="space-y-8">
      <ShowcaseBlock
        title="SAVE TOKENS. BUILD CONNECTIONS."
        subtitle="DX reduces token waste while preserving context so every saved token strengthens your workflow graph."
      >
        <ThreeUpCarousel />
      </ShowcaseBlock>

      <ShowcaseBlock
        title="EVERYTHING IS CONNECTED. NOTHING LIVES IN A SILO."
        subtitle="Editor, terminal, git, docs, and assistant share one coherent runtime context."
      >
        <CenteredThumbCarousel />
      </ShowcaseBlock>

      <ShowcaseBlock
        title="SEE THE SPEED. FEEL THE DIFFERENCE."
        subtitle="Built with Rust for low-latency startup, low memory pressure, and stable high-FPS UI."
      >
        <FullBleedCaptionCarousel />
      </ShowcaseBlock>

      <ShowcaseBlock
        title="MCP APPS. YOUR ENTIRE ECOSYSTEM, INSIDE DX."
        subtitle="DX speaks MCP natively. Connect any MCP-compatible app to expand AI capabilities in one click."
      >
        <SplitMcpCarousel />
      </ShowcaseBlock>

      <ShowcaseBlock
        title="NO INTERNET? NO PROBLEM. DX WORKS EVERYWHERE."
        subtitle="Offline-first runtime with local models and cached context, then seamless sync on reconnect."
      >
        <CardStackOfflineCarousel />
      </ShowcaseBlock>

      <ShowcaseBlock
        title="YOUR KEYBOARD IS YOUR SUPERPOWER."
        subtitle="Shortcuts, automations, and reusable workflows designed for deep engineering flow."
      >
        <TabbedWorkflowCarousel />
      </ShowcaseBlock>
    </div>
  );
}
