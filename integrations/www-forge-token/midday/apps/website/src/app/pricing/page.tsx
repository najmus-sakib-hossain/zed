import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxVideoCarouselSections } from "@/components/dx-video-carousel-sections";

const title = "DX Pricing";
const description =
  "Transparent DX pricing with a generous free tier and scalable plans designed for connected generation and workflow execution.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/pricing`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/pricing`,
  },
};

export default function Page() {
  return (
    <DxVideoCarouselSections
      pageTitle="DX Pricing"
      pageDescription="DX pricing reflects our core thesis: save tokens everywhere, reduce waste, and make advanced workflows economically viable for individuals and teams."
    />
  );
}
