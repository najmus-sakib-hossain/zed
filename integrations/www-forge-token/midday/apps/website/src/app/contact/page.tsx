import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";

const title = "Contact DX";
const description = "Get in touch with the DX team for support, enterprise, partnerships, and product feedback.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/contact` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/contact` },
};

export default function ContactPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8 grid grid-cols-1 lg:grid-cols-2 gap-5">
        <section className="border border-border p-6">
          <h1 className="font-serif text-4xl text-foreground">Get in Touch</h1>
          <p className="mt-3 text-muted-foreground">We usually respond within 24 hours.</p>

          <form className="mt-6 space-y-3">
            <input className="w-full border border-border bg-background p-2.5 text-sm" placeholder="Name" />
            <input className="w-full border border-border bg-background p-2.5 text-sm" placeholder="Email" />
            <select className="w-full border border-border bg-background p-2.5 text-sm">
              <option>General</option>
              <option>Bug Report</option>
              <option>Enterprise</option>
              <option>Partnership</option>
              <option>Press</option>
            </select>
            <textarea className="w-full border border-border bg-background p-2.5 text-sm min-h-[140px]" placeholder="Message" />
            <button type="button" className="border border-foreground px-4 py-2 text-sm text-foreground">Send Message</button>
          </form>
        </section>

        <section className="border border-border p-6">
          <h2 className="text-foreground text-xl">Quick Links</h2>
          <ul className="mt-4 space-y-2 text-sm text-muted-foreground">
            <li>hello@dx.dev</li>
            <li>@dxai on X</li>
            <li>Documentation</li>
            <li>GitHub Issues</li>
          </ul>
          <p className="mt-6 text-sm text-muted-foreground">Office hours: Mon–Fri, 9–6 UTC</p>
        </section>
      </div>
    </div>
  );
}
