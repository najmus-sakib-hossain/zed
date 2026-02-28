import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";
import { Testimonials } from "@/components/testimonials";

const title = "Customer Stories";
const description =
  "See how builders and teams use DX to streamline development workflows and ship faster.";

export const metadata: Metadata = {
  title,
  description,
  keywords: [
    "customer testimonials",
    "user stories",
    "dx reviews",
    "customer success",
    "testimonials",
  ],
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/testimonials`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/testimonials`,
  },
};

export default function Page() {
  return <Testimonials />;
}
