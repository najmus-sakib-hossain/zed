import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { SupportForm } from "@/components/support-form";

const title = "Support";
const description =
  "Get help with DX. Contact our team for assistance with setup, integrations, and workflow issues.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/support`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/support`,
  },
};

export default function SupportPage() {
  return <SupportForm />;
}
