import { z } from 'zod';

export const themeOptionsSchema = z.object({
  dark: z.string(),
  light: z.string(),
});

export const svgSchema = z.object({
  id: z.number().optional(),
  title: z.string().min(1),
  category: z.union([z.string(), z.array(z.string())]),
  route: z.union([z.string(), themeOptionsSchema]),
  wordmark: z.union([z.string(), themeOptionsSchema]).optional(),
  brandUrl: z.string().url().optional(),
  url: z.string().url(),
});

export type SVGSchema = z.infer<typeof svgSchema>;

export function validateSVG(data: unknown) {
  return svgSchema.safeParse(data);
}

export function validateSVGArray(data: unknown) {
  return z.array(svgSchema).safeParse(data);
}
