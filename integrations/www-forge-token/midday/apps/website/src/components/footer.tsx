"use client";

import { cn } from "@midday/ui/cn";
import Link from "next/link";
import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { ThemeToggle } from "./theme-toggle";

type FooterLink = {
  href: string;
  label: string;
  external?: boolean;
};

type FooterSection = {
  title: string;
  links: FooterLink[];
};

const ROTATING_WORDS = ["Enhanced", "Development", "Experience"];

export function Footer() {
  const [currentWordIndex, setCurrentWordIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentWordIndex((prev) => (prev + 1) % ROTATING_WORDS.length);
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  const sections: FooterSection[] = [
    {
      title: "Product",
      links: [
        { href: "/assistant", label: "Assistant" },
        { href: "/forge", label: "Forge" },
        { href: "/media", label: "Media" },
        { href: "/check", label: "Check" },
        { href: "/integrations", label: "Integrations" },
        { href: "/pricing", label: "Pricing" },
        { href: "/download", label: "Download" },
      ],
    },
    {
      title: "Platform",
      links: [
        { href: "/docs", label: "Documentation" },
        { href: "/blog", label: "Blog" },
        { href: "/docs/shortcuts", label: "Shortcuts" },
        { href: "/docs/workflows", label: "Workflows" },
        { href: "/docs/mcp-apps", label: "MCP Apps" },
        { href: "/docs/offline", label: "Offline" },
        { href: "/docs/api", label: "API" },
      ],
    },
    {
      title: "Company",
      links: [
        { href: "/about", label: "About" },
        { href: "/changelog", label: "Changelog" },
        { href: "/contact", label: "Contact" },
        { href: "/security", label: "Security" },
        { href: "/terms", label: "Terms" },
        { href: "https://x.com/dxai", label: "X / Twitter", external: true },
        {
          href: "https://discord.gg/dxai",
          label: "Discord",
          external: true,
        },
      ],
    },
  ];

  return (
    <footer className="bg-background relative overflow-hidden border-t border-border">
      <div className="max-w-[1400px] mx-auto px-4 sm:px-8 py-16 sm:pb-44">
        <div className="grid grid-cols-1 lg:grid-cols-[2fr_1fr] gap-14 lg:gap-20">
          {/* Left side - Links */}
          <div className="grid grid-cols-2 sm:grid-cols-3 gap-8 sm:gap-12">
            {sections.map((section) => (
              <div key={section.title} className="space-y-4">
                <h3 className="font-sans text-sm font-medium text-foreground">
                  {section.title}
                </h3>
                <div className="space-y-2.5">
                  {section.links.map((item) => (
                    <Link
                      key={item.href}
                      href={item.href}
                      target={item.external ? "_blank" : undefined}
                      rel={item.external ? "noopener noreferrer" : undefined}
                      className="font-sans text-sm text-muted-foreground hover:text-foreground transition-colors block"
                    >
                      {item.label}
                    </Link>
                  ))}
                </div>
              </div>
            ))}
          </div>

          {/* Right side - Description */}
          <div className="flex flex-col items-start lg:items-end gap-4">
            <h4 className="font-sans text-lg font-medium text-foreground text-left lg:text-right">
              The Developer Experience You Actually Deserve
            </h4>
            <p className="font-sans text-sm text-muted-foreground text-left lg:text-right max-w-md leading-relaxed">
              DX unifies AI generation, tool calling, media creation, and deep
              workflow integration into one development experience.
            </p>
            <p className="font-sans text-xs text-muted-foreground text-left lg:text-right max-w-md">
              Built in Rust. Optimized with RLM and DX Serializer. Designed for
              developers who ship.
            </p>
          </div>
        </div>

        <div className="my-10">
          <div className="h-px w-full border-t border-border" />
        </div>

        <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6 mb-4">
          {/* Left - System Status */}
          <a
            href="https://dx.openstatus.dev/"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 hover:opacity-80 transition-opacity"
          >
            <span className="font-sans text-sm text-muted-foreground">
              System status:
            </span>
            <span className="font-sans text-sm text-foreground">
              Operational
            </span>
          </a>

          {/* Center - Theme Toggle */}
          <div className="flex justify-center md:absolute md:left-1/2 md:-translate-x-1/2">
            <ThemeToggle />
          </div>

          {/* Right - Copyright */}
          <p className="font-sans text-sm text-muted-foreground">
            © {new Date().getFullYear()} dx. All rights reserved.
          </p>
        </div>
      </div>

      <div className="absolute bottom-0 left-0 sm:left-1/2 sm:-translate-x-1/2 translate-y-[22%] sm:translate-y-[36%] bg-background overflow-hidden pointer-events-none">
        <AnimatePresence mode="wait">
          <motion.h1
            key={currentWordIndex}
            className={cn(
              "font-sans text-[120px] sm:text-[280px] leading-none select-none",
              "text-secondary",
              "[WebkitTextStroke:1px_hsl(var(--muted-foreground))]",
              "[textStroke:1px_hsl(var(--muted-foreground))]",
            )}
            style={{
              WebkitTextStroke: "1px hsl(var(--muted-foreground))",
              color: "hsl(var(--secondary))",
            }}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            transition={{ duration: 0.5, ease: "easeInOut" }}
          >
            {ROTATING_WORDS[currentWordIndex]}
          </motion.h1>
        </AnimatePresence>
      </div>
    </footer>
  );
}
