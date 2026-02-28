import "@/styles/globals.css";
import { cn } from "@midday/ui/cn";
import "@midday/ui/globals.css";
import { Provider as Analytics } from "@midday/events/client";
import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import { NuqsAdapter } from "nuqs/adapters/next/app";
import type { ReactElement } from "react";
import { Footer } from "@/components/footer";
import { Header } from "@/components/header";
import { ThemeProvider } from "@/components/theme-provider";
import { baseUrl } from "./sitemap";

const jetbrainsMono = JetBrains_Mono({
  weight: ["400", "500", "600", "700"],
  subsets: ["latin"],
  display: "swap",
  variable: "--font-jetbrains-mono",
  preload: true,
  adjustFontFallback: true,
  fallback: ["monospace", "Courier New"],
});

export const metadata: Metadata = {
  metadataBase: new URL(baseUrl),
  title: {
    default: "Enhance Your Development Experience | DX",
    template: "%s | DX",
  },
  description:
    "DX is the universal development experience — native on every OS, 100+ AI providers, offline-first, Rust-powered, with Forge VCS, Traffic Security, Check scoring, and unified media generation.",
  openGraph: {
    title: "Enhance Your Development Experience | DX",
    description:
      "DX is the universal development experience — native on every OS, 100+ AI providers, offline-first, Rust-powered, with Forge VCS, Traffic Security, Check scoring, and unified media generation.",
    url: baseUrl,
    siteName: "DX",
    locale: "en_US",
    type: "website",
    images: [
      {
        url: "https://cdn.dx.ai/opengraph-image-v1.jpg",
        width: 800,
        height: 600,
      },
      {
        url: "https://cdn.dx.ai/opengraph-image-v1.jpg",
        width: 1800,
        height: 1600,
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Enhance Your Development Experience | DX",
    description:
      "DX is the universal development experience — native on every OS, 100+ AI providers, offline-first, Rust-powered, with Forge VCS, Traffic Security, Check scoring, and unified media generation.",
    images: [
      {
        url: "https://cdn.dx.ai/opengraph-image-v1.jpg",
        width: 800,
        height: 600,
      },
      {
        url: "https://cdn.dx.ai/opengraph-image-v1.jpg",
        width: 1800,
        height: 1600,
      },
    ],
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
};

export const viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)" },
    { media: "(prefers-color-scheme: dark)" },
  ],
};

const jsonLd = {
  "@context": "https://schema.org",
  "@type": "Organization",
  name: "DX",
  url: "https://dx.ai",
  logo: "https://cdn.dx.ai/logo.png",
  sameAs: [
    "https://x.com/dxai",
    "https://discord.gg/dxai",
    "https://github.com/dxai",
  ],
  description:
    "DX is the universal development experience — native on every OS, 100+ AI providers, offline-first, Rust-powered, with Forge VCS, Traffic Security, Check scoring, and unified media generation.",
};

export default function Layout({ children }: { children: ReactElement }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <link rel="preconnect" href="https://cdn.dx.ai" />
        <link rel="dns-prefetch" href="https://cdn.dx.ai" />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify(jsonLd).replace(/</g, "\\u003c"),
          }}
        />
      </head>
      <body
        className={cn(
          `${jetbrainsMono.variable} font-mono`,
          "bg-background overflow-x-hidden font-mono antialiased",
        )}
      >
        <NuqsAdapter>
          <ThemeProvider
            attribute="class"
            defaultTheme="system"
            enableSystem
            disableTransitionOnChange
          >
            <Header />
            <main className="max-w-[1400px] mx-auto px-4 overflow-hidden md:overflow-visible">
              {children}
            </main>
            <Footer />
            <Analytics />
          </ThemeProvider>
        </NuqsAdapter>
      </body>
    </html>
  );
}
