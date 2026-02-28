'use client';

import { useEffect, useState } from 'react';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Home, Heart, Package } from 'lucide-react';
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar';
import { ScrollArea } from '@/components/ui/scroll-area';
import { svgs } from '@/data/svgs';
import { Separator } from './ui/separator';
import { getAvailableIconPacks } from '@/lib/icon-data-loader';
import { ICON_PACK_COUNTS, TOTAL_ICONS } from '@/lib/icon-pack-counts';

const mainNav = [
  { title: 'Home', icon: Home, href: '/' },
  { title: 'Favorites', icon: Heart, href: '/favorites' },
];

export function AppSidebar() {
  const pathname = usePathname();
  // Initialize with all packs immediately from static data to prevent flashing
  const allPackNames = ['svgl', ...Object.keys(ICON_PACK_COUNTS).sort()];
  const [iconPacks] = useState<string[]>(allPackNames);
  const [packCounts] = useState<Record<string, number>>({ svgl: svgs.length, ...ICON_PACK_COUNTS });

  return (
    <Sidebar>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              <Link href="/" className="flex items-center space-x-2 w-full rounded-md h-8 px-2">
                <Image src="/logo.svg" alt="DX" width={16} height={16} className="w-4 h-4" />
                <span className="font-bold">DX</span>
              </Link>
              {mainNav.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton asChild isActive={pathname === item.href}>
                    <Link href={item.href}>
                      <item.icon className="h-4 w-4" />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
        <Separator className="max-w-[90%] mx-auto" />
        <SidebarGroup>
          <SidebarGroupContent>
            <ScrollArea className="h-[calc(100vh-12rem)] pl-1 pr-2 rounded-md">
              <div className="flex h-8 items-center justify-between px-2 mb-1">
                <div className="flex items-center gap-2">
                  <Package className="h-4 w-4" />
                  <span className="text-sm font-medium">Total ({iconPacks.length})</span>
                </div>
                <span className="text-xs text-muted-foreground font-mono rounded-lg border p-1 px-2 bg-primary-foreground font-mono">
                  {(TOTAL_ICONS + svgs.length).toLocaleString()}
                </span>
              </div>
              <SidebarMenu>
                {iconPacks.map((pack) => {
                  const count = packCounts[pack] || 0;
                  const displayName = pack === 'svgl'
                    ? 'SVGL'
                    : pack.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');

                  return (
                    <SidebarMenuItem key={pack}>
                      <button
                        className="w-full h-8 p-2 flex items-center justify-between flex-row text-muted-foreground hover:text-primary hover:bg-accent rounded-md transition-colors"
                        onClick={() => {
                          // Navigate to home page and trigger pack selection
                          if (pathname !== '/') {
                            window.location.href = `/?pack=${pack}`;
                          } else {
                            window.dispatchEvent(new CustomEvent('selectIconPack', { detail: pack }));
                          }
                        }}
                      >
                        <span className="text-sm w-32 text-left truncate">{displayName}</span>
                        <span className="ml-auto text-xs rounded-lg border p-1 px-2 bg-primary-foreground font-mono shrink-0">
                          {count}
                        </span>
                      </button>
                    </SidebarMenuItem>
                  );
                })}
              </SidebarMenu>
            </ScrollArea>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
