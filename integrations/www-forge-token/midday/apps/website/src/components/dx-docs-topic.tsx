import Link from "next/link";

export function DxDocsTopic({
  title,
  description,
  bullets,
}: {
  title: string;
  description: string;
  bullets: string[];
}) {
  return (
    <div className="min-h-[calc(100vh-180px)] pt-32 pb-20">
      <div className="max-w-[1000px] mx-auto px-4 sm:px-8">
        <div className="border border-border p-6 sm:p-8">
          <p className="text-xs uppercase tracking-wide text-muted-foreground">DX Docs</p>
          <h1 className="mt-3 font-serif text-4xl text-foreground">{title}</h1>
          <p className="mt-3 text-muted-foreground max-w-3xl">{description}</p>

          <ul className="mt-6 space-y-2">
            {bullets.map((item) => (
              <li key={item} className="text-sm text-muted-foreground border border-border p-3">
                {item}
              </li>
            ))}
          </ul>

          <div className="mt-7 flex flex-wrap gap-3 text-sm">
            <Link href="/docs" className="border border-border px-3 py-2 text-foreground">Docs Home</Link>
            <Link href="/integrations" className="border border-border px-3 py-2 text-foreground">Integrations</Link>
          </div>
        </div>
      </div>
    </div>
  );
}
