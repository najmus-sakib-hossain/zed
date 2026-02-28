"use client";

import { motion } from "motion/react";
import { 
  Apple,
  Monitor,
  Laptop,
  Smartphone,
  Tablet,
  Watch,
  Tv,
  Chrome,
  Code2,
  Globe,
  Server,
  Copy,
  Check,
  MoreHorizontal,
} from "lucide-react";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@midday/ui/hover-card";
import { Button } from "@midday/ui/button";
import { useState } from "react";

interface PlatformDownload {
  name: string;
  icon: any;
  label: string;
  description: string;
  downloadMethod: "curl" | "extension" | "store" | "web";
  curlCommand?: string;
  instructions?: string[];
}

const platforms: PlatformDownload[] = [
  { 
    name: "macOS", 
    icon: Apple, 
    label: "macOS",
    description: "Native desktop app for macOS",
    downloadMethod: "curl",
    curlCommand: "curl -fsSL https://dx.ai/install.sh | sh",
    instructions: [
      "The curl command automatically detects your OS and installs all DX extensions",
      "Installs desktop app, browser extensions, and IDE plugins",
      "Or download the .dmg from dx.ai/download",
      "Supports macOS 11.0 and later"
    ]
  },
  { 
    name: "Windows", 
    icon: Monitor, 
    label: "Windows",
    description: "Native desktop app for Windows",
    downloadMethod: "curl",
    curlCommand: "irm https://dx.ai/install.ps1 | iex",
    instructions: [
      "The PowerShell command automatically detects your OS and installs all DX extensions",
      "Installs desktop app, browser extensions, and IDE plugins",
      "Or download the .exe installer from dx.ai/download",
      "Supports Windows 10 and later"
    ]
  },
  { 
    name: "Linux", 
    icon: Laptop, 
    label: "Linux",
    description: "Native desktop app for Linux",
    downloadMethod: "curl",
    curlCommand: "curl -fsSL https://dx.ai/install.sh | sh",
    instructions: [
      "The curl command automatically detects your OS and installs all DX extensions",
      "Installs desktop app, browser extensions, and IDE plugins",
      "Supports Ubuntu, Debian, Fedora, Arch",
      "AppImage and Snap packages available"
    ]
  },
  { 
    name: "Android", 
    icon: Smartphone, 
    label: "Android",
    description: "Mobile app for Android devices",
    downloadMethod: "store",
    instructions: [
      "Download from Google Play Store",
      "Search for 'DX - Developer Experience'",
      "Supports Android 8.0 and later"
    ]
  },
  { 
    name: "iOS", 
    icon: Smartphone, 
    label: "iOS",
    description: "Mobile app for iPhone and iPad",
    downloadMethod: "store",
    instructions: [
      "Download from Apple App Store",
      "Search for 'DX - Developer Experience'",
      "Supports iOS 14.0 and later"
    ]
  },
  { 
    name: "ChromeOS", 
    icon: Chrome, 
    label: "ChromeOS",
    description: "Native app for Chromebooks",
    downloadMethod: "store",
    instructions: [
      "Download from Chrome Web Store",
      "Or use the web app at app.dx.ai",
      "Full offline support available"
    ]
  },
  { 
    name: "Tablet", 
    icon: Tablet, 
    label: "Tablet",
    description: "Optimized for tablet devices",
    downloadMethod: "store",
    instructions: [
      "Available on App Store and Play Store",
      "Tablet-optimized UI with split-screen",
      "Supports iPad and Android tablets"
    ]
  },
  { 
    name: "Watch", 
    icon: Watch, 
    label: "Watch",
    description: "Companion app for smartwatches",
    downloadMethod: "store",
    instructions: [
      "Companion app for Apple Watch and Wear OS",
      "Quick voice commands and notifications",
      "Requires phone app to be installed"
    ]
  },
  { 
    name: "TV", 
    icon: Tv, 
    label: "TV",
    description: "Companion app for smart TVs",
    downloadMethod: "store",
    instructions: [
      "Available for Apple TV and Android TV",
      "Remote control and voice commands",
      "Perfect for presentations and demos"
    ]
  },
  { 
    name: "Browser", 
    icon: Globe, 
    label: "Browser",
    description: "Browser extension for all major browsers",
    downloadMethod: "extension",
    instructions: [
      "Download the extension file from dx.ai/download",
      "Open your browser's extension settings",
      "Enable 'Developer mode' and load the extension",
      "Supports Chrome, Firefox, Safari, Edge, Brave"
    ]
  },
  { 
    name: "IDE", 
    icon: Code2, 
    label: "IDE",
    description: "Extensions for popular IDEs",
    downloadMethod: "extension",
    instructions: [
      "Download from your IDE's marketplace",
      "VS Code, JetBrains, Neovim, Zed supported",
      "Or manually install from dx.ai/download"
    ]
  },
  { 
    name: "VPS", 
    icon: Server, 
    label: "VPS",
    description: "Deploy on your own server",
    downloadMethod: "curl",
    curlCommand: "curl -fsSL https://dx.ai/install-server.sh | sh",
    instructions: [
      "Run the server installation script",
      "Supports Docker and bare metal deployment",
      "Full documentation at docs.dx.ai/vps"
    ]
  },
  { 
    name: "More", 
    icon: MoreHorizontal, 
    label: "More",
    description: "Additional platform support",
    downloadMethod: "web",
    instructions: [
      "FreeBSD, OpenBSD, NetBSD support",
      "Raspberry Pi and ARM devices",
      "Custom builds for specialized systems",
      "Docker containers for any platform",
      "Web app accessible from any device",
      "Visit dx.ai/download for all options"
    ]
  },
];

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Button
      variant="ghost"
      size="sm"
      className="h-8 px-2"
      onClick={handleCopy}
    >
      {copied ? (
        <Check className="w-4 h-4 text-green-500" />
      ) : (
        <Copy className="w-4 h-4" />
      )}
    </Button>
  );
}

export function PlatformDownloadCards() {
  return (
    <motion.div
      className="mt-12 mb-8"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, delay: 0.2 }}
    >
      <div className="flex flex-wrap items-center justify-center gap-4">
        {platforms.map((platform, index) => {
          const Icon = platform.icon;
          return (
            <HoverCard key={platform.name} openDelay={200}>
              <HoverCardTrigger asChild>
                <motion.div
                  className="flex flex-col items-center gap-2"
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ 
                    duration: 0.3, 
                    delay: 0.3 + index * 0.05,
                    type: "spring",
                    stiffness: 400,
                    damping: 25,
                  }}
                >
                  <motion.button
                    className="flex items-center justify-center w-14 h-14 rounded-lg border border-border bg-background hover:bg-muted/50 hover:border-primary/50 transition-colors cursor-pointer"
                    whileHover={{ scale: 1.1, y: -2 }}
                    whileTap={{ scale: 0.95 }}
                    transition={{ type: "spring", stiffness: 400, damping: 20 }}
                  >
                    <Icon className="w-6 h-6 text-foreground" />
                  </motion.button>
                  <span className="text-xs text-muted-foreground">
                    {platform.label}
                  </span>
                </motion.div>
              </HoverCardTrigger>
              <HoverCardContent 
                className="w-80 p-4 bg-background border border-border shadow-lg"
                side="bottom"
                align="center"
              >
                <div className="space-y-3">
                  <div className="flex items-start gap-3">
                    <div className="flex items-center justify-center w-10 h-10 rounded-lg border border-border bg-muted/30">
                      <Icon className="w-5 h-5 text-foreground" />
                    </div>
                    <div className="flex-1">
                      <h4 className="text-sm font-medium text-foreground">
                        {platform.label}
                      </h4>
                      <p className="text-xs text-muted-foreground mt-0.5">
                        {platform.description}
                      </p>
                    </div>
                  </div>

                  {platform.curlCommand && (
                    <div className="space-y-2">
                      <p className="text-xs font-medium text-foreground">
                        Quick Install:
                      </p>
                      <div className="flex items-center gap-2 p-2 rounded-md bg-muted/50 border border-border">
                        <code className="flex-1 text-xs text-foreground font-mono overflow-x-auto">
                          {platform.curlCommand}
                        </code>
                        <CopyButton text={platform.curlCommand} />
                      </div>
                      <p className="text-xs text-muted-foreground italic">
                        Automatically detects your OS and installs all DX extensions
                      </p>
                    </div>
                  )}

                  {platform.instructions && platform.instructions.length > 0 && (
                    <div className="space-y-2">
                      <p className="text-xs font-medium text-foreground">
                        {platform.downloadMethod === "extension" ? "Installation:" : "Download:"}
                      </p>
                      <ul className="space-y-1.5">
                        {platform.instructions.map((instruction, idx) => (
                          <li key={idx} className="text-xs text-muted-foreground flex items-start gap-2">
                            <span className="text-primary mt-0.5">•</span>
                            <span className="flex-1">{instruction}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}

                  <div className="pt-2 border-t border-border">
                    <Button asChild variant="outline" size="sm" className="w-full h-8 text-xs">
                      <a href="/download">View All Downloads →</a>
                    </Button>
                  </div>
                </div>
              </HoverCardContent>
            </HoverCard>
          );
        })}
      </div>
    </motion.div>
  );
}
