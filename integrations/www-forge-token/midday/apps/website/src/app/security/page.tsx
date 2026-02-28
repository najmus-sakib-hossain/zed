import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";

const title = "DX Security";
const description = "Security and privacy architecture for DX, including local-first execution, cloud safeguards, and MCP permission controls.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: { title, description, type: "website", url: `${baseUrl}/security` },
  twitter: { card: "summary_large_image", title, description },
  alternates: { canonical: `${baseUrl}/security` },
};

const sections = [
  {
    title: "Local-First Architecture",
    body: "DX processes workflows locally by default. Your code does not leave your machine unless cloud features are explicitly enabled.",
  },
  {
    title: "Cloud AI Security",
    body: "TLS 1.3 transport, strict isolation, no training on private payloads, and short-lived processing windows.",
  },
  {
    title: "MCP App Security",
    body: "Permission-based MCP access, explicit app authorization, revocation controls, and sandboxed execution boundaries.",
  },
  {
    title: "Offline Mode Security",
    body: "Encrypted local storage, local model execution, and no mandatory network round-trips for core productivity.",
  },
  {
    title: "Vulnerability Disclosure",
    body: "Report issues via security@dx.dev. Responsible disclosure path and coordinated response policy.",
  },
];

export default function SecurityPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[900px] mx-auto px-4 sm:px-8">
        <h1 className="font-serif text-4xl text-foreground">Your Code. Your Data. Your Control.</h1>
        <p className="mt-3 text-muted-foreground">Security architecture and policy for DX runtime and integrations.</p>

        <div className="mt-8 space-y-4">
          {sections.map((section) => (
            <section key={section.title} className="border border-border p-5">
              <h2 className="text-foreground text-xl">{section.title}</h2>
              <p className="mt-2 text-sm text-muted-foreground">{section.body}</p>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
