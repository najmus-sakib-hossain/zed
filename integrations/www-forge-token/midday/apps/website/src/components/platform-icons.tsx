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
  MonitorSmartphone,
} from "lucide-react";

const platforms = [
  { name: "macOS", icon: Apple, label: "macOS" },
  { name: "Windows", icon: Monitor, label: "Windows" },
  { name: "Linux", icon: Laptop, label: "Linux" },
  { name: "Android", icon: Smartphone, label: "Android" },
  { name: "iOS", icon: Apple, label: "iOS" },
  { name: "ChromeOS", icon: Chrome, label: "ChromeOS" },
  { name: "Tablet", icon: Tablet, label: "Tablet" },
  { name: "Watch", icon: Watch, label: "Watch" },
  { name: "TV", icon: Tv, label: "TV" },
  { name: "Browser", icon: Globe, label: "Browser" },
  { name: "IDE", icon: Code2, label: "IDE" },
];

export function PlatformIcons() {
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
            <motion.div
              key={platform.name}
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
          );
        })}
      </div>
    </motion.div>
  );
}
