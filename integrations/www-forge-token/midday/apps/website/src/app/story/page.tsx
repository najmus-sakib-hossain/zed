import type { Metadata } from "next";
import Image from "next/image";
import { baseUrl } from "@/app/sitemap";

const title = "Story";
const description =
  "Why we built DX. Learn about our mission to unify AI workflows, tooling, and media creation into one development experience.";

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    title,
    description,
    type: "website",
    url: `${baseUrl}/story`,
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
  },
  alternates: {
    canonical: `${baseUrl}/story`,
  },
};

export default function StoryPage() {
  return (
    <div className="min-h-screen">
      <div className="pt-32 pb-16 sm:pb-24">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="space-y-12">
            {/* Title */}
            <div className="space-y-4 text-center">
              <h1 className="font-serif text-3xl lg:text-3xl xl:text-3xl 2xl:text-3xl 3xl:text-4xl leading-tight lg:leading-tight xl:leading-[1.3] text-foreground">
                Why we started DX
              </h1>
            </div>

            {/* Content */}
            <div className="prose prose-sm sm:prose-base max-w-none space-y-8 font-sans text-foreground">
              {/* The problem */}
              <section className="space-y-4">
                <h2 className="font-sans text-base text-foreground">
                  The problem
                </h2>
                <p className="text-muted-foreground leading-relaxed">
                  Building software and media products shouldn't require ten
                  disconnected tools just to ship one outcome.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  We kept hitting the same wall: code generation in one app,
                  research in another, automations in another, and media work
                  spread everywhere else. Context was constantly lost between
                  tools, and shipping became slower than it should be.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  Most products optimize only one slice of the workflow. DX was
                  created to connect the whole system: generation, tooling,
                  orchestration, and media in one runtime.
                </p>
              </section>

              {/* Divider */}
              <div className="flex items-center justify-center py-8">
                <div className="h-px w-full max-w-xs border-t border-border" />
              </div>

              {/* The idea */}
              <section className="space-y-4">
                <h2 className="font-sans text-base text-foreground">
                  The idea
                </h2>
                <p className="text-muted-foreground leading-relaxed">
                  We didn't want another isolated AI app. We wanted a system
                  that works for you.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  DX is built around one principle: if workflows are connected,
                  users move faster with less effort. Code, research, tool
                  calls, documents, images, video, and 3D workflows should not
                  live in silos.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  Instead of forcing users to glue everything manually, DX keeps
                  context shared across operations. The same intent can power
                  Ask, Agent, Plan, Search, Study, and Research workflows.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  We built DX in Rust because speed and efficiency matter when
                  workflows get complex. Token optimization, offline support,
                  and cross-platform execution are all part of that foundation.
                </p>
              </section>

              {/* Divider */}
              <div className="flex items-center justify-center py-8">
                <div className="h-px w-full max-w-xs border-t border-border" />
              </div>

              {/* What we're focused on */}
              <section className="space-y-4">
                <h2 className="font-sans text-base text-foreground">
                  What we're focused on
                </h2>
                <p className="text-muted-foreground leading-relaxed">
                  DX is built for developers, creators, and teams who want one
                  system that can keep up with real production work.
                </p>
                <p className="text-muted-foreground leading-relaxed">
                  We focus on:
                </p>
                <ul className="list-disc list-inside space-y-2 text-muted-foreground ml-4">
                  <li>Reducing context switching across tools</li>
                  <li>Keeping AI workflows spec-driven and reliable</li>
                  <li>Making advanced capabilities usable by everyone</li>
                  <li>Improving speed, token efficiency, and cost control</li>
                  <li>Building systems that stay practical in daily use</li>
                </ul>
                <p className="text-muted-foreground leading-relaxed mt-4">
                  Our goal is simple: when you use DX, you should spend less
                  time stitching tools together and more time shipping.
                </p>
                <p className="text-muted-foreground leading-relaxed font-medium">
                  Your workflow should explain itself.
                </p>
              </section>
            </div>

            {/* Founders Image */}
            <div className="w-full space-y-3">
              <Image
                src="/founders.png"
                alt="Founders"
                width={1200}
                height={450}
                className="w-full h-[350px] sm:h-[450px] object-cover object-center"
                priority
              />
              <div className="text-left">
                <p className="font-sans text-sm text-primary">
                  Pontus & Viktor
                </p>
                <p className="font-sans text-sm text-muted-foreground">
                  Founders, DX
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
