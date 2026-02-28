'use client';

import { useState, useEffect } from 'react';
import { Settings, Code2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';

interface Framework {
  id: string;
  name: string;
  logo: string;
  category: 'web' | 'backend' | 'mobile' | 'desktop';
}

export function SettingsDialog() {
  const [selectedFrameworks, setSelectedFrameworks] = useState<string[]>([]);
  const [optimizeSvgs, setOptimizeSvgs] = useState(true);

  useEffect(() => {
    const saved = localStorage.getItem('selectedFrameworks');
    if (saved) {
      setSelectedFrameworks(JSON.parse(saved));
    } else {
      // Default selections
      setSelectedFrameworks(['react', 'nextjs', 'vue']);
    }
    
    const optimize = localStorage.getItem('optimizeSvgs');
    if (optimize !== null) {
      setOptimizeSvgs(optimize === 'true');
    }
  }, []);

  const frameworks: Framework[] = [
    // Web Frameworks
    { id: 'react', name: 'React', logo: '/svgl/react_dark.svg', category: 'web' },
    { id: 'nextjs', name: 'Next.js', logo: '/svgl/nextjs_icon_dark.svg', category: 'web' },
    { id: 'vue', name: 'Vue', logo: '/svgl/vue.svg', category: 'web' },
    { id: 'nuxt', name: 'Nuxt', logo: '/svgl/nuxt.svg', category: 'web' },
    { id: 'angular', name: 'Angular', logo: '/svgl/angular.svg', category: 'web' },
    { id: 'svelte', name: 'Svelte', logo: '/svgl/svelte.svg', category: 'web' },
    { id: 'solid', name: 'Solid', logo: '/svgl/solidjs.svg', category: 'web' },
    { id: 'qwik', name: 'Qwik', logo: '/svgl/qwik.svg', category: 'web' },
    { id: 'astro', name: 'Astro', logo: '/svgl/astro-icon-dark.svg', category: 'web' },
    // Backend Frameworks
    { id: 'django', name: 'Django', logo: '/svgl/django.svg', category: 'backend' },
    { id: 'flask', name: 'Flask', logo: '/svgl/flask-dark.svg', category: 'backend' },
    { id: 'fastapi', name: 'FastAPI', logo: '/svgl/fastapi.svg', category: 'backend' },
    { id: 'actix', name: 'Actix Web', logo: '/svgl/rust.svg', category: 'backend' },
    { id: 'rocket', name: 'Rocket', logo: '/svgl/rust.svg', category: 'backend' },
    { id: 'axum', name: 'Axum', logo: '/svgl/rust.svg', category: 'backend' },
    { id: 'gin', name: 'Gin', logo: '/svgl/golang.svg', category: 'backend' },
    { id: 'echo', name: 'Echo', logo: '/svgl/golang.svg', category: 'backend' },
    { id: 'fiber', name: 'Fiber', logo: '/svgl/golang.svg', category: 'backend' },
    { id: 'laravel', name: 'Laravel', logo: '/svgl/laravel.svg', category: 'backend' },
    // Mobile Platforms
    { id: 'react-native', name: 'React Native', logo: '/svgl/react_dark.svg', category: 'mobile' },
    { id: 'flutter', name: 'Flutter', logo: '/svgl/flutter.svg', category: 'mobile' },
    { id: 'swift', name: 'Swift (iOS)', logo: '/svgl/swift.svg', category: 'mobile' },
    { id: 'kotlin', name: 'Kotlin (Android)', logo: '/svgl/kotlin.svg', category: 'mobile' },
    // Desktop Platforms
    { id: 'electron', name: 'Electron', logo: '/svgl/electron.svg', category: 'desktop' },
    { id: 'tauri', name: 'Tauri', logo: '/svgl/tauri.svg', category: 'desktop' },
  ];

  const categories = [
    { id: 'web', name: 'Web', icon: '🌐' },
    { id: 'backend', name: 'Backend', icon: '⚙️' },
    { id: 'mobile', name: 'Mobile', icon: '📱' },
    { id: 'desktop', name: 'Desktop', icon: '💻' },
  ];

  const toggleFramework = (id: string) => {
    setSelectedFrameworks((prev) =>
      prev.includes(id) ? prev.filter((f) => f !== id) : [...prev, id]
    );
  };

  const handleSave = () => {
    localStorage.setItem('selectedFrameworks', JSON.stringify(selectedFrameworks));
    localStorage.setItem('optimizeSvgs', String(optimizeSvgs));
  };

  const handleReset = () => {
    setSelectedFrameworks(['react', 'nextjs', 'vue']);
    setOptimizeSvgs(true);
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon" title="Settings">
          <Settings className="h-5 w-5" />
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-4xl max-h-[85vh] p-0">
        <DialogHeader className="px-6 pt-6 pb-4 border-b">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Code2 className="h-5 w-5 text-primary" />
            </div>
            <div>
              <DialogTitle className="text-xl">Settings</DialogTitle>
              <DialogDescription className="text-sm mt-1">
                Customize your preferred frameworks and options
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="px-6 py-4 space-y-6">
          <div className="space-y-3">
            <Label className="text-base font-semibold">Preferred Frameworks</Label>
            <p className="text-sm text-muted-foreground">
              Select frameworks to show in copy modal (you can still access all frameworks)
            </p>

            <Tabs defaultValue="web" className="w-full">
              <TabsList className="grid w-full grid-cols-4">
                {categories.map((category) => (
                  <TabsTrigger key={category.id} value={category.id} className="gap-2">
                    <span>{category.icon}</span>
                    <span className="hidden sm:inline">{category.name}</span>
                  </TabsTrigger>
                ))}
              </TabsList>

              <ScrollArea className="h-[40vh] mt-4">
                {categories.map((category) => {
                  const categoryFrameworks = frameworks.filter((f) => f.category === category.id);

                  return (
                    <TabsContent key={category.id} value={category.id}>
                      <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                        {categoryFrameworks.map((framework) => (
                          <button
                            key={framework.id}
                            onClick={() => toggleFramework(framework.id)}
                            className={cn(
                              'flex items-center gap-3 p-3 rounded-lg border transition-all',
                              selectedFrameworks.includes(framework.id)
                                ? 'border-primary bg-primary/5'
                                : 'border-border hover:border-primary/50'
                            )}
                          >
                            <img
                              src={framework.logo}
                              alt={framework.name}
                              className="w-6 h-6 object-contain"
                              suppressHydrationWarning
                            />
                            <span className="text-sm font-medium">{framework.name}</span>
                          </button>
                        ))}
                      </div>
                    </TabsContent>
                  );
                })}
              </ScrollArea>
            </Tabs>
          </div>

          <div className="flex items-center justify-between space-x-2 pt-4 border-t">
            <div className="space-y-0.5">
              <Label htmlFor="optimize-svgs" className="text-base font-semibold">
                Optimize SVGs
              </Label>
              <p className="text-sm text-muted-foreground">
                Use SVGO to optimize SVGs when copying (reduces file size)
              </p>
            </div>
            <Switch
              id="optimize-svgs"
              checked={optimizeSvgs}
              onCheckedChange={setOptimizeSvgs}
            />
          </div>
        </div>

        <div className="flex items-center justify-end gap-2 px-6 py-4 border-t">
          <Button variant="outline" onClick={handleReset}>
            Reset
          </Button>
          <Button onClick={handleSave}>Save Changes</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
