import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { DxVideoCarouselSections } from "@/components/dx-video-carousel-sections";

const title = "DX Integrations";
const description =
  "DX integrations connect providers, protocols, tools, and workflows in one runtime with shared context and orchestration.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/integrations`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/integrations`,
  },
};

export default function Page() {
  return (
    <DxVideoCarouselSections
      pageTitle="DX Integrations"
      pageDescription="DX meets you where you work: IDEs, browsers, creative tools, and protocol-based systems all connect through one unified execution graph."
    />
  );
}
