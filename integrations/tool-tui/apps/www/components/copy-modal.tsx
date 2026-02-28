'use client';

import { useState } from 'react';
import { Check, Copy, Code2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';

interface CopyModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  svgUrl: string;
  iconName: string;
}

interface CopyFormat {
  id: string;
  name: string;
  logo?: string;
  category: 'web' | 'mobile' | 'desktop' | 'backend';
  code: string;
}

export function CopyModal({ open, onOpenChange, svgUrl, iconName }: CopyModalProps) {
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const handleCopy = async (format: CopyFormat) => {
    try {
      await navigator.clipboard.writeText(format.code);
      setCopiedId(format.id);
      toast.success(`Copied ${format.name} format!`);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (error) {
      toast.error('Failed to copy');
    }
  };

  const formats: CopyFormat[] = [
    // Web Frameworks
    {
      id: 'react',
      name: 'React',
      logo: '/svgl/react_dark.svg',
      category: 'web',
      code: `import { ${iconName} } from '@/components/icons';

export function MyComponent() {
  return <${iconName} className="w-6 h-6" />;
}`,
    },
    {
      id: 'nextjs',
      name: 'Next.js',
      logo: '/svgl/nextjs_icon_dark.svg',
      category: 'web',
      code: `import Image from 'next/image';

export default function Page() {
  return <Image src="${svgUrl}" alt="${iconName}" width={24} height={24} />;
}`,
    },
    {
      id: 'vue',
      name: 'Vue',
      logo: '/svgl/vue.svg',
      category: 'web',
      code: `<template>
  <img src="${svgUrl}" alt="${iconName}" class="w-6 h-6" />
</template>`,
    },
    {
      id: 'nuxt',
      name: 'Nuxt',
      logo: '/svgl/nuxt.svg',
      category: 'web',
      code: `<template>
  <NuxtImg src="${svgUrl}" alt="${iconName}" width="24" height="24" />
</template>`,
    },
    {
      id: 'angular',
      name: 'Angular',
      logo: '/svgl/angular.svg',
      category: 'web',
      code: `<img src="${svgUrl}" alt="${iconName}" width="24" height="24" />`,
    },
    {
      id: 'svelte',
      name: 'Svelte',
      logo: '/svgl/svelte.svg',
      category: 'web',
      code: `<script>
  const iconUrl = '${svgUrl}';
</script>

<img src={iconUrl} alt="${iconName}" class="w-6 h-6" />`,
    },
    {
      id: 'solid',
      name: 'Solid',
      logo: '/svgl/solidjs.svg',
      category: 'web',
      code: `import { createSignal } from 'solid-js';

function MyComponent() {
  return <img src="${svgUrl}" alt="${iconName}" class="w-6 h-6" />;
}`,
    },
    {
      id: 'qwik',
      name: 'Qwik',
      logo: '/svgl/qwik.svg',
      category: 'web',
      code: `import { component$ } from '@builder.io/qwik';

export default component$(() => {
  return <img src="${svgUrl}" alt="${iconName}" width={24} height={24} />;
});`,
    },
    {
      id: 'astro',
      name: 'Astro',
      logo: '/svgl/astro-icon-dark.svg',
      category: 'web',
      code: `---
const iconUrl = '${svgUrl}';
---
<img src={iconUrl} alt="${iconName}" width="24" height="24" />`,
    },
    // Python Frameworks
    {
      id: 'django',
      name: 'Django',
      logo: '/svgl/django.svg',
      category: 'backend',
      code: `<!-- In your template -->
<img src="{% static '${svgUrl}' %}" alt="${iconName}" width="24" height="24">`,
    },
    {
      id: 'flask',
      name: 'Flask',
      logo: '/svgl/flask-dark.svg',
      category: 'backend',
      code: `<!-- In your template -->
<img src="{{ url_for('static', filename='${svgUrl}') }}" alt="${iconName}" width="24" height="24">`,
    },
    {
      id: 'fastapi',
      name: 'FastAPI',
      logo: '/svgl/fastapi.svg',
      category: 'backend',
      code: `from fastapi import FastAPI
from fastapi.responses import FileResponse

@app.get("/icon")
async def get_icon():
    return FileResponse("${svgUrl}")`,
    },
    // Rust Frameworks
    {
      id: 'actix',
      name: 'Actix Web',
      logo: '/svgl/rust.svg',
      category: 'backend',
      code: `use actix_web::{web, App, HttpResponse, HttpServer};
use actix_files::Files;

async fn icon() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(include_str!("${svgUrl}"))
}`,
    },
    {
      id: 'rocket',
      name: 'Rocket',
      logo: '/svgl/rust.svg',
      category: 'backend',
      code: `#[get("/icon")]
fn icon() -> &'static str {
    include_str!("${svgUrl}")
}`,
    },
    {
      id: 'axum',
      name: 'Axum',
      logo: '/svgl/rust.svg',
      category: 'backend',
      code: `use axum::{response::Html, routing::get, Router};

async fn icon() -> Html<&'static str> {
    Html(include_str!("${svgUrl}"))
}`,
    },
    // Go Frameworks
    {
      id: 'gin',
      name: 'Gin',
      logo: '/svgl/golang.svg',
      category: 'backend',
      code: `func getIcon(c *gin.Context) {
    c.File("${svgUrl}")
}`,
    },
    {
      id: 'echo',
      name: 'Echo',
      logo: '/svgl/golang.svg',
      category: 'backend',
      code: `func getIcon(c echo.Context) error {
    return c.File("${svgUrl}")
}`,
    },
    {
      id: 'fiber',
      name: 'Fiber',
      logo: '/svgl/golang.svg',
      category: 'backend',
      code: `app.Get("/icon", func(c *fiber.Ctx) error {
    return c.SendFile("${svgUrl}")
})`,
    },
    // PHP Frameworks
    {
      id: 'laravel',
      name: 'Laravel',
      logo: '/svgl/laravel.svg',
      category: 'backend',
      code: `<!-- In Blade template -->
<img src="{{ asset('${svgUrl}') }}" alt="${iconName}" width="24" height="24">`,
    },
    // Mobile Platforms
    {
      id: 'react-native',
      name: 'React Native',
      logo: '/svgl/react_dark.svg',
      category: 'mobile',
      code: `import { Image } from 'react-native';

<Image source={{ uri: '${svgUrl}' }} style={{ width: 24, height: 24 }} />`,
    },
    {
      id: 'flutter',
      name: 'Flutter',
      logo: '/svgl/flutter.svg',
      category: 'mobile',
      code: `import 'package:flutter_svg/flutter_svg.dart';

SvgPicture.network(
  '${svgUrl}',
  width: 24,
  height: 24,
)`,
    },
    {
      id: 'swift',
      name: 'Swift (iOS)',
      logo: '/svgl/swift.svg',
      category: 'mobile',
      code: `import SwiftUI

Image("${iconName}")
    .resizable()
    .frame(width: 24, height: 24)`,
    },
    {
      id: 'kotlin',
      name: 'Kotlin (Android)',
      logo: '/svgl/kotlin.svg',
      category: 'mobile',
      code: `import android.widget.ImageView

imageView.setImageResource(R.drawable.${iconName.toLowerCase()})`,
    },
    // Desktop Platforms
    {
      id: 'electron',
      name: 'Electron',
      logo: '/svgl/electron.svg',
      category: 'desktop',
      code: `const { nativeImage } = require('electron');

const icon = nativeImage.createFromPath('${svgUrl}');`,
    },
    {
      id: 'tauri',
      name: 'Tauri',
      logo: '/svgl/tauri.svg',
      category: 'desktop',
      code: `<!-- In your HTML -->
<img src="${svgUrl}" alt="${iconName}" width="24" height="24" />`,
    },
  ];

  const categories = [
    { id: 'web', name: 'Web', icon: '🌐' },
    { id: 'backend', name: 'Backend', icon: '⚙️' },
    { id: 'mobile', name: 'Mobile', icon: '📱' },
    { id: 'desktop', name: 'Desktop', icon: '💻' },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl max-h-[85vh] p-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Code2 className="h-5 w-5 text-primary" />
            </div>
            <div>
              <DialogTitle className="text-xl">Copy Icon Code</DialogTitle>
              <DialogDescription className="text-sm mt-1">
                Select your framework to get the implementation code
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <Tabs defaultValue="web" className="w-full">
          <div className="px-6 pt-2">
            <TabsList className="grid w-full grid-cols-4">
              {categories.map((category) => (
                <TabsTrigger key={category.id} value={category.id} className="gap-2">
                  <span>{category.icon}</span>
                  <span className="hidden sm:inline">{category.name}</span>
                </TabsTrigger>
              ))}
            </TabsList>
          </div>

          <ScrollArea className="h-[55vh] px-6 pb-6">
            {categories.map((category) => {
              const categoryFormats = formats.filter((f) => f.category === category.id);
              
              return (
                <TabsContent key={category.id} value={category.id} className="mt-4">
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
                    {categoryFormats.map((format) => (
                      <div
                        key={format.id}
                        className="group relative rounded-lg border bg-card hover:border-primary/50 transition-all duration-200"
                      >
                        <div className="flex items-center justify-between p-4 pb-3">
                          <div className="flex items-center gap-2.5">
                            {format.logo && (
                              <div className="w-6 h-6 flex items-center justify-center">
                                <img
                                  src={format.logo}
                                  alt={format.name}
                                  className="w-full h-full object-contain"
                                  suppressHydrationWarning
                                />
                              </div>
                            )}
                            <span className="font-semibold text-sm">{format.name}</span>
                          </div>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleCopy(format)}
                            className={cn(
                              "h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity",
                              copiedId === format.id && "opacity-100"
                            )}
                          >
                            {copiedId === format.id ? (
                              <Check className="h-4 w-4 text-green-500" strokeWidth={2.5} />
                            ) : (
                              <Copy className="h-4 w-4" />
                            )}
                          </Button>
                        </div>
                        <div className="px-4 pb-4">
                          <pre className="text-xs bg-muted/50 p-3 rounded-md overflow-x-auto border font-mono leading-relaxed">
                            <code className="text-foreground/90">{format.code}</code>
                          </pre>
                        </div>
                      </div>
                    ))}
                  </div>
                </TabsContent>
              );
            })}
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
