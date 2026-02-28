import type { MetadataRoute } from "next";
import { categories, getAllSlugs } from "@/data/apps";
import { getAllCompetitorSlugs } from "@/data/competitors";
import { getBlogPosts } from "@/lib/blog";
import { getAllDocSlugs } from "@/lib/docs";

export const baseUrl = "https://dx.ai";

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const lastModified = new Date().toISOString().split("T")[0];

  // Static routes
  const staticRoutes = [
    "",
    "/about",
    "/assistant",
    "/blog",
    "/changelog",
    "/check",
    "/compare",
    "/contact",
    "/customers",
    "/docs",
    "/docs/api",
    "/docs/getting-started",
    "/docs/mcp-apps",
    "/docs/offline",
    "/docs/shortcuts",
    "/docs/workflows",
    "/download",
    "/forge",
    "/integrations",
    "/media",
    "/mcp",
    "/mcp/chatgpt",
    "/mcp/claude",
    "/mcp/copilot",
    "/mcp/cursor",
    "/mcp/make",
    "/mcp/n8n",
    "/mcp/opencode",
    "/mcp/perplexity",
    "/mcp/raycast",
    "/mcp/zapier",
    "/policy",
    "/pricing",
    "/security",
    "/story",
    "/support",
    "/terms",
    "/updates",
  ].map((route) => ({
    url: `${baseUrl}${route}`,
    lastModified,
  }));

  // Blog/updates posts
  const blogPosts = getBlogPosts().map((post) => ({
    url: `${baseUrl}/updates/${post.slug}`,
    lastModified: post.metadata.publishedAt,
  }));

  // Integration pages
  const integrations = getAllSlugs().map((slug) => ({
    url: `${baseUrl}/integrations/${slug}`,
    lastModified,
  }));

  // Integration category pages
  const integrationCategories = categories
    .filter((c) => c.id !== "all")
    .map((c) => ({
      url: `${baseUrl}/integrations/category/${c.id}`,
      lastModified,
    }));

  // Documentation pages
  const docPages = getAllDocSlugs().map((slug) => ({
    url: `${baseUrl}/docs/${slug}`,
    lastModified,
  }));

  // Comparison pages
  const comparisonPages = getAllCompetitorSlugs().map((slug) => ({
    url: `${baseUrl}/compare/${slug}`,
    lastModified,
  }));

  return [
    ...staticRoutes,
    ...blogPosts,
    ...integrations,
    ...integrationCategories,
    ...docPages,
    ...comparisonPages,
  ];
}
