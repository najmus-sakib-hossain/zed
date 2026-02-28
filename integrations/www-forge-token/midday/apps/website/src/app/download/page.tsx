import type { Metadata } from "next";
import { baseUrl } from "@/app/sitemap";

const title = "Download DX";
const description =
  "Download DX for Mac. Your workflows, always one click away. Access your business data directly from your desktop.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/download`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/download`,
  },
};

export default function DownloadPage() {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[1100px] mx-auto px-4 sm:px-8 space-y-8">
        <section className="border border-border p-6 sm:p-8">
          <p className="text-xs uppercase tracking-wide text-muted-foreground">Download</p>
          <h1 className="mt-3 font-serif text-4xl text-foreground">Download DX</h1>
          <p className="mt-4 text-muted-foreground max-w-3xl">
            Available for macOS, Linux, and Windows. Start free and keep the full connected development flow.
          </p>
        </section>

        <section className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="border border-foreground p-5">
            <h2 className="text-foreground text-lg">macOS</h2>
            <ul className="mt-3 space-y-1 text-sm text-muted-foreground">
              <li>Apple Silicon</li>
              <li>Intel</li>
              <li>brew install dx</li>
            </ul>
            <a href="#" className="mt-5 inline-block text-sm text-foreground border border-border px-3 py-2">Download</a>
          </div>

          <div className="border border-border p-5">
            <h2 className="text-foreground text-lg">Linux</h2>
            <ul className="mt-3 space-y-1 text-sm text-muted-foreground">
              <li>.deb (Ubuntu)</li>
              <li>.rpm (Fedora)</li>
              <li>.AppImage</li>
            </ul>
            <a href="#" className="mt-5 inline-block text-sm text-foreground border border-border px-3 py-2">Download</a>
          </div>

          <div className="border border-border p-5">
            <h2 className="text-foreground text-lg">Windows</h2>
            <ul className="mt-3 space-y-1 text-sm text-muted-foreground">
              <li>.exe installer</li>
              <li>.msi package</li>
              <li>winget install dx</li>
            </ul>
            <a href="#" className="mt-5 inline-block text-sm text-foreground border border-border px-3 py-2">Download</a>
          </div>
        </section>

        <section className="border border-border p-6 sm:p-8">
          <h3 className="font-serif text-2xl text-foreground">System Requirements</h3>
          <ul className="mt-4 space-y-2 text-sm text-muted-foreground">
            <li>macOS 12+, Linux kernel 5.10+, Windows 10/11</li>
            <li>4GB RAM minimum (DX baseline runtime is far lower)</li>
            <li>200MB disk space</li>
          </ul>
          <div className="mt-6 border border-border p-3 text-sm text-foreground">
            $ curl -fsSL https://dx.dev/install | sh
          </div>
        </section>
      </div>
    </div>
  );
}
