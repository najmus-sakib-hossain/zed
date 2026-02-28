import { readFile } from 'fs/promises';
import { NextRequest, NextResponse } from 'next/server';
import { join } from 'path';
import { getIconData, iconToSVG, iconToHTML } from '@iconify/utils';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ pack: string; name: string }> }
) {
  const { pack, name } = await params;
  
  try {
    // SVGL icons: serve directly from public/svgl/*.svg files
    if (pack === 'svgl') {
      // Try exact filename first
      let svgPath = join(process.cwd(), 'public', 'svgl', `${name}.svg`);
      try {
        let svgContent = await readFile(svgPath, 'utf-8');
        
        // Fix viewBox issues: remove width/height attributes
        svgContent = svgContent.replace(/\s+width="[^"]*"/g, '');
        svgContent = svgContent.replace(/\s+height="[^"]*"/g, '');
        
        // Fix Affinity SVGs: content has transform="translate(-1528)" and coordinates 1603-2477
        // Original viewBox is 0 0 1024 1024, but content is at 1603-2477 (874px wide)
        // After translate(-1528), content is at 75-949 which fits in 0-1024
        if (name === 'affinity_designer' || name === 'affinity_photo' || name === 'affinity_publisher') {
          // The transform is correct, just need to ensure viewBox shows the full range
          // Content after transform: 75 to 949 (874px), centered in 1024px viewBox
          // Keep original viewBox but ensure no clipping
        }
        
        return new NextResponse(svgContent, {
          headers: {
            'Content-Type': 'image/svg+xml',
            'Cache-Control': 'public, max-age=31536000, immutable',
          },
        });
      } catch {
        // Try with underscores converted to hyphens
        const altName = name.replace(/_/g, '-');
        svgPath = join(process.cwd(), 'public', 'svgl', `${altName}.svg`);
        try {
          let svgContent = await readFile(svgPath, 'utf-8');
          svgContent = svgContent.replace(/\s+width="[^"]*"/g, '');
          svgContent = svgContent.replace(/\s+height="[^"]*"/g, '');
          
          return new NextResponse(svgContent, {
            headers: {
              'Content-Type': 'image/svg+xml',
              'Cache-Control': 'public, max-age=31536000, immutable',
            },
          });
        } catch {
          // Try adding _light suffix for theme variants
          const lightPath = join(process.cwd(), 'public', 'svgl', `${name}_light.svg`);
          try {
            let svgContent = await readFile(lightPath, 'utf-8');
            svgContent = svgContent.replace(/\s+width="[^"]*"/g, '');
            svgContent = svgContent.replace(/\s+height="[^"]*"/g, '');
            
            return new NextResponse(svgContent, {
              headers: {
                'Content-Type': 'image/svg+xml',
                'Cache-Control': 'public, max-age=31536000, immutable',
              },
            });
          } catch {
            // Try with hyphens and _light suffix
            const hyphenLightPath = join(process.cwd(), 'public', 'svgl', `${altName}_light.svg`);
            try {
              let svgContent = await readFile(hyphenLightPath, 'utf-8');
              svgContent = svgContent.replace(/\s+width="[^"]*"/g, '');
              svgContent = svgContent.replace(/\s+height="[^"]*"/g, '');
              
              return new NextResponse(svgContent, {
                headers: {
                  'Content-Type': 'image/svg+xml',
                  'Cache-Control': 'public, max-age=31536000, immutable',
                },
              });
            } catch {
              return new NextResponse('Icon not found', { status: 404 });
            }
          }
        }
      }
    }
    
    // Other packs: use JSON files from public/icons/
    const jsonPath = join(process.cwd(), 'public', 'icons', `${pack}.json`);
    const jsonContent = await readFile(jsonPath, 'utf-8');
    const iconSet = JSON.parse(jsonContent);
    
    // Use iconify utils to properly extract and render icon
    const iconData = getIconData(iconSet, name);
    
    if (!iconData) {
      return new NextResponse('Icon not found', { status: 404 });
    }
    
    // Use iconify's iconToSVG to generate proper viewBox and attributes
    const renderData = iconToSVG(iconData, {
      height: '1em',
    });
    
    // Convert to HTML with proper attributes
    let svg = iconToHTML(renderData.body, renderData.attributes);
    
    // Use currentColor for theme support
    svg = svg.replace(/fill="(?!none)[^"]*"/g, 'fill="currentColor"');
    svg = svg.replace(/stroke="(?!none)[^"]*"/g, 'stroke="currentColor"');
    
    return new NextResponse(svg, {
      headers: {
        'Content-Type': 'image/svg+xml',
        'Cache-Control': 'public, max-age=31536000, immutable',
      },
    });
  } catch (error) {
    console.error(`Failed to load icon ${pack}/${name}:`, error);
    return new NextResponse('Internal Server Error', { status: 500 });
  }
}

// Extract SVG body from .llm format (for dx, lucide, solar packs only)
function extractIconSVG(content: string, iconName: string, pack: string): string | null {
  const lines = content.split('\n');
  const searchKey = `icons.${iconName}(`;
  
  for (const line of lines) {
    if (line.includes(searchKey)) {
      // Extract body parameter - can be quoted body="..." or unquoted body=...
      const bodyIdx = line.indexOf('body=');
      if (bodyIdx === -1) continue;
      
      const afterEquals = line.substring(bodyIdx + 5);
      let body = '';
      
      if (afterEquals.startsWith('"')) {
        // Quoted format: body="..."
        const contentStart = 1; // Skip opening quote
        let escaped = false;
        
        for (let i = contentStart; i < afterEquals.length; i++) {
          const ch = afterEquals[i];
          if (escaped) {
            body += ch;
            escaped = false;
          } else if (ch === '\\') {
            escaped = true;
            body += ch;
          } else if (ch === '"') {
            // Found end of body
            break;
          } else {
            body += ch;
          }
        }
      } else {
        // Unquoted format: body=<svg...> width=...
        // Body ends at next attribute (width=, height=) or closing paren
        const match = afterEquals.match(/^(.+?)(?:\s+(?:width|height)=|\))/);
        if (match) {
          body = match[1].trim();
        }
      }
      
      if (!body) continue;
      
      // Unescape the body content
      let svg = body.replace(/\\"/g, '"').replace(/\\n/g, '\n');
      
      // Use currentColor for theme support
      svg = svg.replace(/fill="(?!none)[^"]*"/g, 'fill="currentColor"');
      svg = svg.replace(/stroke="(?!none)[^"]*"/g, 'stroke="currentColor"');
      
      // Wrap in SVG tag if not already wrapped - use 512x512 for .llm format icons
      if (!svg.includes('<svg')) {
        svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" fill="currentColor">${svg}</svg>`;
      } else {
        // Ensure existing SVG tags use currentColor
        svg = svg.replace(/<svg([^>]*?)>/i, (match, attrs) => {
          if (!attrs.includes('fill=')) {
            return `<svg${attrs} fill="currentColor">`;
          }
          return match;
        });
      }
      
      return svg;
    }
  }
  
  return null;
}
