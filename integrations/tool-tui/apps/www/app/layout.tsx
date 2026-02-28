import type { Metadata } from 'next';
import { JetBrains_Mono } from 'next/font/google';
import { Github } from 'lucide-react';
import { Toaster } from 'sonner';
import { Analytics } from '@vercel/analytics/react';
import { SpeedInsights } from '@vercel/speed-insights/next';
import { Providers } from '@/components/providers';
import { ThemeToggle } from '@/components/theme-toggle';
import { CommandPalette } from '@/components/command-palette';
import { SettingsDialog } from '@/components/settings-dialog';
import { Button } from '@/components/ui/button';
import { SidebarProvider, SidebarTrigger, SidebarInset } from '@/components/ui/sidebar';
import { AppSidebar } from '@/components/app-sidebar';
import { WasmLoader } from '@/components/wasm-loader';
import './globals.css';

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-jetbrains-mono',
  display: 'swap',
});

export const metadata: Metadata = {
  title: 'DX Icons - Binary-First Icon Library',
  description: 'Search, copy and download 579+ SVG icons instantly. Built with Rust and WebAssembly.',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning className={jetbrainsMono.variable}>
      <body className="antialiased font-sans">
        <Providers>
          <SidebarProvider>
            <AppSidebar />
            <SidebarInset>
              <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
                <div className="flex h-14 items-center px-4">
                  <div className="flex items-center gap-2">
                    <SidebarTrigger />
                  </div>
                  <div className="flex flex-1 items-center justify-end space-x-2">
                    <Button variant="ghost" size="icon" asChild>
                      <a
                        href="https://github.com/dx-rs/dx"
                        target="_blank"
                        rel="noopener noreferrer"
                        title="GitHub"
                      >
                        <Github className="h-5 w-5" />
                      </a>
                    </Button>
                    <SettingsDialog />
                    <ThemeToggle />
                  </div>
                </div>
              </header>
              <main className="flex-1 p-4">{children}</main>
            </SidebarInset>
          </SidebarProvider>
          <CommandPalette />
          <WasmLoader />
          <Toaster position="bottom-right" />
          <Analytics />
          <SpeedInsights />
        </Providers>
      </body>
    </html>
  );
}
