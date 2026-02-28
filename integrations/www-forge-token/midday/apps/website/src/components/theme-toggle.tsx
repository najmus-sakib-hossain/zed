"use client";

import { Moon, Sun, Monitor } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { motion } from "motion/react";

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return (
      <div className="inline-flex items-center gap-1 rounded-full border border-border bg-background p-1 h-10">
        <div className="w-20 h-8" />
      </div>
    );
  }

  const themes = [
    { value: "light", icon: Sun, label: "Light" },
    { value: "dark", icon: Moon, label: "Dark" },
    { value: "system", icon: Monitor, label: "System" },
  ];

  return (
    <div className="inline-flex items-center gap-1 rounded-full border border-border bg-muted/30 p-1 relative bg-red-500">
      {themes.map((themeOption) => {
        const Icon = themeOption.icon;
        const isActive = theme === themeOption.value;
        
        return (
          <button
            key={themeOption.value}
            onClick={() => setTheme(themeOption.value)}
            className={`
              relative z-10 flex items-center gap-2 px-3 py-1.5 rounded-full text-sm font-medium transition-colors
              ${isActive ? "text-foreground" : "text-muted-foreground hover:text-foreground"}
            `}
            aria-label={`Switch to ${themeOption.label} theme`}
          >
            {isActive && (
              <motion.div
                layoutId="theme-indicator"
                className="absolute inset-0 bg-background border border-border rounded-full"
                transition={{ type: "spring", stiffness: 400, damping: 30 }}
              />
            )}
            <Icon className="w-4 h-4 relative z-10" />
            <span className="relative z-10 hidden sm:inline">{themeOption.label}</span>
          </button>
        );
      })}
    </div>
  );
}
