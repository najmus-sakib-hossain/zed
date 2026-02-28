import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxVideoCarouselSections } from "@/components/dx-video-carousel-sections";

const title = "DX Assistant";
const description =
  "DX Assistant is part of one connected development runtime — code, tools, research, media, and automation in a single context.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/assistant`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/assistant`,
  },
};

export default function Page() {
  return (
    <DxVideoCarouselSections
      pageTitle="DX Assistant"
      pageDescription="Assistant in DX is not an isolated chatbot. It is a connected execution layer that shares state with your workflows, tools, generation engines, and automation pipelines."
    />
  );
}
