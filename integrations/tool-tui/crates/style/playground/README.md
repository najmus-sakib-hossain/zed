# DX Style Playground

Test all production features of dx-style.

## Usage

```bash
cd crates/style/playground
../../../target/release/dx-style
```

Or with watch mode:
```bash
DX_WATCH=1 ../../../target/release/dx-style
```

## Features Tested

1. **Basic Utilities** - Flexbox, Grid, Spacing
2. **Animations** - fade-in, slide-up, bounce, pulse, composed
3. **Arbitrary Values** - Custom widths, colors, gradients
4. **Auto-Grouping** - Repeated patterns automatically grouped
5. **Responsive** - Breakpoints (sm, md, lg, xl)
6. **States** - hover, focus, active
7. **Typography** - Font sizes, weights
8. **Spacing** - Padding, margin, sizing
9. **Colors** - Full color palette with opacity
10. **Borders** - Border widths, colors, radius
11. **Transforms** - Rotate, scale, skew
12. **Filters** - Blur, brightness, contrast

## Expected Output

- `style.css` - Generated CSS file
- `.dx/style/` - Binary cache and metadata
- Auto-grouped classes will be rewritten in HTML

## Verification

Open `index.html` in a browser to see all features working.
