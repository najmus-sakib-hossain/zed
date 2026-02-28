# DX AI Agent Face

An interactive, minimalist face component for the DX AI agent with smooth animations and extensible emotion system.

## Design Philosophy

The DX AI face is designed to be:
- **Minimalist**: Simple geometric shapes (semi-circles for eyes, curves for mouth)
- **Cute & Friendly**: Approachable, welcoming design inspired by classic emoji aesthetics
- **Smooth**: Natural animations using spring physics
- **Scalable**: Easy to add new emotions through a registry system

## Visual Design

### Default State (Happy)
- **Eyes**: Semi-circle shapes (⌒ ⌒) positioned at 35% from top
- **Mouth**: Gentle smile curve (⌣) positioned at 58% from top
- **Spacing**: Well-balanced, creating a friendly, approachable look
- **Border**: Clean circular border with subtle glow effect

### Features

1. **Auto-blinking**: Eyes blink naturally every 3-5 seconds
2. **Mouse tracking**: Eyes subtly follow cursor movement (limited range for natural look)
3. **Hover effect**: Face scales up slightly with enhanced glow
4. **Ambient particles**: Subtle floating particles around the face
5. **Smooth transitions**: All animations use spring physics

## Emotion System

### Architecture

Emotions are defined in a registry object, making it trivial to add new expressions:

```typescript
const EMOTIONS: Record<string, Emotion> = {
  happy: {
    eyes: { shape: "semicircle", top: "35%", scaleY: 1 },
    mouth: { path: "M 30 58 Q 50 68 70 58" }
  },
  // Add more emotions here - no component code changes needed!
};
```

### Available Emotions

1. **happy** (default): Gentle smile, semi-circle eyes
2. **neutral**: Slight smile, slightly smaller eyes
3. **excited**: Big smile, circular eyes
4. **surprised**: O-shaped mouth, wide circular eyes
5. **thinking**: Straight mouth, narrowed eyes

### Adding New Emotions

To add a new emotion, simply add an entry to the `EMOTIONS` object:

```typescript
myNewEmotion: {
  name: "myNewEmotion",
  eyes: {
    shape: "semicircle" | "circle" | "line",
    top: "35%",  // Vertical position
    scaleY: 1,   // Vertical scale
  },
  mouth: {
    path: "M 30 60 Q 50 65 70 60",  // SVG path
  },
  eyebrows: {  // Optional
    left: "...",
    right: "...",
  }
}
```

## Usage

```tsx
import { DxAiFace } from "@/components/dx-ai-face";

// Default (200px, happy, interactive)
<DxAiFace />

// Custom size
<DxAiFace size={280} />

// Different emotion
<DxAiFace emotion="excited" />

// Disable interactivity
<DxAiFace interactive={false} />

// Auto emotion changes
<DxAiFace autoEmote={true} />

// All options
<DxAiFace 
  size={280}
  emotion="happy"
  interactive={true}
  autoEmote={false}
  className="my-custom-class"
/>
```

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `size` | `number` | `200` | Size in pixels (width and height) |
| `emotion` | `string` | `"happy"` | Emotion name from EMOTIONS registry |
| `interactive` | `boolean` | `true` | Enable mouse tracking |
| `autoEmote` | `boolean` | `false` | Randomly change emotions every 5s |
| `className` | `string` | `""` | Additional CSS classes |

## Implementation Details

- Built with Framer Motion for smooth, performant animations
- Uses SVG for scalable, crisp rendering at any size
- Spring physics for natural eye movement
- Optimized mouse tracking with limited range
- Automatic blinking with randomized intervals (3-5 seconds)
- GPU-accelerated transforms for smooth performance

## Design Inspiration

Based on minimalist emoji expressions:
- Simple geometric shapes
- Clean, modern aesthetic
- Friendly, approachable character
- Smooth, natural animations
- Interactive feedback for engagement

## Future Extensibility

The emotion system is designed for easy expansion:
- Add emotions without touching component logic
- Could load emotions from external config/API
- Support for custom emotion objects
- Easy to add new features (eyebrows, accessories, etc.)
