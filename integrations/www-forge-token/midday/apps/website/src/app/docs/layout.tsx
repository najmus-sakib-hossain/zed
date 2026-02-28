import type { ReactNode } from "react";
import { Suspense } from "react";
import { DocsChatProvider } from "@/components/docs/docs-chat-provider";

export const metadata = {
  title: "Documentation",
  description:
    "Learn how to use DX for connected AI workflows and development operations",
};

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <Suspense fallback={children}>
      <DocsChatProvider>{children}</DocsChatProvider>
    </Suspense>
  );
}
