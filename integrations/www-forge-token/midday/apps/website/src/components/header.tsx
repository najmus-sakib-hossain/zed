"use client";

import { Button } from "@midday/ui/button";
import { cn } from "@midday/ui/cn";
import { Icons } from "@midday/ui/icons";
import { motion } from "motion/react";
import Link from "next/link";
import { useState } from "react";

interface HeaderProps {
  transparent?: boolean;
  hideMenuItems?: boolean;
}

const productDropdown = [
  { href: "/assistant", label: "Assistant", desc: "6 modes: Ask, Agent, Plan, Search, Study, Research" },
  // { href: "/forge", label: "Forge", desc: "Unlimited VCS for every media type" },
  { href: "/media", label: "Media", desc: "5,000+ fonts · 219 icon sets · 1M+ icons" },
  { href: "/check", label: "Check", desc: "500-point security & quality scoring" },
  { href: "/security", label: "Security", desc: "Green/Yellow/Red traffic safety system" },
];

const docsNavigation = [
  { href: "/docs/getting-started", label: "Getting Started" },
  { href: "/docs/shortcuts", label: "Shortcuts" },
  { href: "/docs/workflows", label: "Workflows" },
  { href: "/docs/mcp-apps", label: "MCP Apps" },
  { href: "/docs/offline", label: "Offline" },
  { href: "/docs/api", label: "API" },
];

const navigation = [
  { href: "/", label: "Home", dropdown: null },
  { href: "/product", label: "Product", dropdown: "product" },
  { href: "/integrations", label: "Integrations", dropdown: null },
  { href: "/pricing", label: "Pricing", dropdown: null },
  { href: "/docs", label: "Docs", dropdown: "docs" },
  { href: "/download", label: "Download", dropdown: null },
];

export function Header({ transparent = false, hideMenuItems = false }: HeaderProps) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <header className="fixed top-0 left-0 right-0 z-50">
      <div
        className={cn(
          "py-3 xl:py-4 px-4 sm:px-4 md:px-4 lg:px-4 xl:px-6 2xl:px-8",
          "flex items-center justify-between border-b border-border",
          transparent ? "bg-background/60 backdrop-blur-md" : "bg-background/80 backdrop-blur-md",
        )}
      >
        {/* Logo */}
        <Link
          href="/"
          className="flex items-center gap-2 hover:opacity-80 transition-opacity"
          aria-label="DX - Go to homepage"
          onClick={() => setIsOpen(false)}
        >
          <div className="w-6 h-6">
            <Icons.LogoSmall className="w-full h-full text-foreground" />
          </div>
          <span className="font-sans text-base text-foreground">dx</span>
        </Link>

        {/* Desktop nav */}
        {!hideMenuItems ? (
          <nav className="hidden xl:flex items-center gap-1 absolute left-1/2 -translate-x-1/2">
            {/* Home */}
            <Link
              href="/"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Home
            </Link>

            {/* Product dropdown */}
            <div className="relative group">
              <button
                type="button"
                className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
              >
                Product
                <svg className="w-3 h-3 transition-transform duration-200 group-hover:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </button>
              <div className="pointer-events-none opacity-0 -translate-y-1 transition-all duration-200 group-hover:pointer-events-auto group-hover:opacity-100 group-hover:translate-y-0 absolute top-full left-0 pt-2 z-50">
                <div className="w-72 border border-border bg-background backdrop-blur-md p-2 shadow-lg rounded-md">
                  {productDropdown.map((item) => (
                    <Link
                      key={item.href}
                      href={item.href}
                      className="flex flex-col gap-0.5 rounded-sm px-3 py-2.5 hover:bg-muted transition-colors"
                    >
                      <span className="text-sm text-foreground">{item.label}</span>
                      <span className="text-xs text-muted-foreground">{item.desc}</span>
                    </Link>
                  ))}
                </div>
              </div>
            </div>

            {/* Integrations */}
            <Link
              href="/integrations"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Integrations
            </Link>

            {/* Pricing */}
            <Link
              href="/providers"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Providers
            </Link>
            <Link
              href="/tools"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Tools
            </Link>
            <Link
              href="/forge"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Forge
            </Link>

            {/* Docs dropdown */}
            <div className="relative group">
              <Link
                href="/docs"
                className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
              >
                Docs
                <svg className="w-3 h-3 transition-transform duration-200 group-hover:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                </svg>
              </Link>
              <div className="pointer-events-none opacity-0 -translate-y-1 transition-all duration-200 group-hover:pointer-events-auto group-hover:opacity-100 group-hover:translate-y-0 absolute top-full left-1/2 -translate-x-1/2 pt-2 z-50">
                <div className="w-56 border border-border bg-background backdrop-blur-md p-2 shadow-lg rounded-md">
                  {docsNavigation.map((docsItem) => (
                    <Link
                      key={docsItem.href}
                      href={docsItem.href}
                      className="block rounded-sm px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                    >
                      {docsItem.label}
                    </Link>
                  ))}
                </div>
              </div>
            </div>

            {/* Download */}
            <Link
              href="/download"
              className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Download
            </Link>
          </nav>
        ) : null}

        {/* Desktop CTAs */}
        <div className="hidden xl:flex items-center gap-2">
          <Button asChild variant="outline" className="h-9 px-4">
            <Link href="/docs">Read Blogs</Link>
          </Button>
          <Button asChild className="btn-inverse h-9 px-4">
            <Link href="/download">Explore DX</Link>
          </Button>
        </div>

        {/* Mobile hamburger */}
        <button
          type="button"
          className="xl:hidden p-2 text-muted-foreground hover:text-foreground"
          aria-label="Toggle menu"
          onClick={() => setIsOpen((prev) => !prev)}
        >
          {isOpen ? <Icons.Close /> : <Icons.Menu />}
        </button>
      </div>

      {/* Mobile menu */}
      {isOpen ? (
        <motion.div
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          className="xl:hidden border-b border-border bg-background backdrop-blur-md"
        >
          <div className="px-4 py-4 flex flex-col gap-1">
            <Link href="/" className="text-sm text-muted-foreground hover:text-foreground px-2 py-2 block" onClick={() => setIsOpen(false)}>Home</Link>

            {/* Product sub-links */}
            <div>
              <p className="text-xs text-muted-foreground px-2 pt-2 pb-1 uppercase tracking-wide">Product</p>
              {productDropdown.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="text-sm text-muted-foreground hover:text-foreground px-4 py-1.5 block"
                  onClick={() => setIsOpen(false)}
                >
                  {item.label}
                </Link>
              ))}
            </div>

            <Link href="/integrations" className="text-sm text-muted-foreground hover:text-foreground px-2 py-2 block" onClick={() => setIsOpen(false)}>Integrations</Link>
            <Link href="/pricing" className="text-sm text-muted-foreground hover:text-foreground px-2 py-2 block" onClick={() => setIsOpen(false)}>Pricing</Link>

            {/* Docs sub-links */}
            <div>
              <Link href="/docs" className="text-sm text-muted-foreground hover:text-foreground px-2 py-2 block" onClick={() => setIsOpen(false)}>Docs</Link>
              <div className="pl-5 pb-1 flex flex-col">
                {docsNavigation.map((docsItem) => (
                  <Link
                    key={docsItem.href}
                    href={docsItem.href}
                    className="text-xs text-muted-foreground hover:text-foreground py-1"
                    onClick={() => setIsOpen(false)}
                  >
                    {docsItem.label}
                  </Link>
                ))}
              </div>
            </div>

            <Link href="/download" className="text-sm text-muted-foreground hover:text-foreground px-2 py-2 block" onClick={() => setIsOpen(false)}>Download</Link>

            <div className="pt-3 grid grid-cols-2 gap-2">
              <Button asChild variant="outline" className="h-9">
                <Link href="/docs" onClick={() => setIsOpen(false)}>Read Docs</Link>
              </Button>
              <Button asChild className="btn-inverse h-9">
                <Link href="/download" onClick={() => setIsOpen(false)}>Download DX ▶</Link>
              </Button>
            </div>
          </div>
        </motion.div>
      ) : null}
    </header>
  );
}
