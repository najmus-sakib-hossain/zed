import { notFound } from 'next/navigation';
import { svgs } from '@/data/svgs';
import { SvgCard } from '@/components/svg-card';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export function generateStaticParams() {
  const categories = new Set<string>();
  svgs.forEach((svg) => {
    if (Array.isArray(svg.category)) {
      svg.category.forEach((c) => categories.add(c.toLowerCase()));
    } else {
      categories.add(svg.category.toLowerCase());
    }
  });
  return Array.from(categories).map((category) => ({ category }));
}

export default function CategoryPage({ params }: { params: { category: string } }) {
  const categoryName = decodeURIComponent(params.category);
  
  const filteredSvgs = svgs.filter((svg) => {
    if (Array.isArray(svg.category)) {
      return svg.category.some((c) => c.toLowerCase() === categoryName);
    }
    return svg.category.toLowerCase() === categoryName;
  });

  if (filteredSvgs.length === 0) {
    notFound();
  }

  const displayCategory = categoryName.charAt(0).toUpperCase() + categoryName.slice(1);

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center justify-between">
            <span>{displayCategory}</span>
            <span className="text-muted-foreground font-mono text-base">
              {filteredSvgs.length} logos
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
            {filteredSvgs.map((svg) => (
              <SvgCard key={svg.id} svg={svg} />
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
