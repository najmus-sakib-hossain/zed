export interface OptimizeSvgOptions {
  removeViewBox?: boolean;
  removeDimensions?: boolean;
}

export async function optimizeSvg(
  svgString: string,
  options: OptimizeSvgOptions = {}
): Promise<string> {
  try {
    // Basic SVG optimization without svgo (client-side safe)
    let optimized = svgString;
    
    // Remove XML declaration
    optimized = optimized.replace(/<\?xml[^?]*\?>/g, '');
    
    // Remove comments
    optimized = optimized.replace(/<!--[\s\S]*?-->/g, '');
    
    // Remove unnecessary whitespace
    optimized = optimized.replace(/\s+/g, ' ').trim();
    
    // Remove dimensions if requested
    if (options.removeDimensions) {
      optimized = optimized.replace(/\s*(width|height)="[^"]*"/g, '');
    }
    
    return optimized;
  } catch (error) {
    console.error('SVG optimization failed:', error);
    return svgString;
  }
}

export async function fetchAndOptimizeSvg(url: string): Promise<string> {
  try {
    const response = await fetch(url);
    const svgText = await response.text();
    return await optimizeSvg(svgText);
  } catch (error) {
    console.error('Failed to fetch and optimize SVG:', error);
    throw error;
  }
}
