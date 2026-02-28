# Website Updates - March 3, 2026 Launch

## Summary
Updated website with launch date, social links, and enhanced platform download section with interactive hover cards.

## Changes Made

### 1. Launch Date Updated
- **Old**: February 24, 2026
- **New**: March 3, 2026
- **Files Updated**:
  - `midday/apps/website/src/components/startpage.tsx` (2 locations)
    - Hero section launch date
    - Waitlist section launch date

### 2. Social Links - LinkedIn → Discord
Replaced all LinkedIn references with Discord across the website.

**Files Updated**:
- `midday/apps/website/src/components/footer.tsx`
  - Changed link from `https://www.linkedin.com/company/dx-ai` to `https://discord.gg/dxai`
  - Updated label from "LinkedIn" to "Discord"

- `midday/apps/website/src/app/layout.tsx`
  - Updated JSON-LD schema sameAs array
  - Changed from `https://linkedin.com/company/dx-ai` to `https://discord.gg/dxai`

### 3. Enhanced Platform Download Cards
Created a new interactive component with hover cards showing detailed download information.

**New File**: `midday/apps/website/src/components/platform-download-cards.tsx`

**Features**:
- 12 platforms supported:
  - macOS, Windows, Linux (with curl commands)
  - Android, iOS (App Store links)
  - ChromeOS, Tablet, Watch, TV (Store downloads)
  - Browser, IDE (Extension installation)
  - VPS (Server deployment)

**Hover Card Details**:
- Platform icon and description
- Download method specific to each platform
- For desktop apps (macOS, Windows, Linux, VPS):
  - Curl command with copy button
  - Alternative download methods
  - System requirements
- For browser/IDE extensions:
  - Manual installation instructions
  - Step-by-step guide for loading extensions
  - Supported browsers/IDEs list
- For mobile/companion apps:
  - App store information
  - Device compatibility
  - Feature highlights

**Interactive Features**:
- Hover to see detailed download card
- Copy button for curl commands (with visual feedback)
- Smooth animations and transitions
- Responsive design
- Link to full download page

**Platform-Specific Instructions**:

1. **macOS/Windows/Linux/VPS**: 
   - One-line curl/PowerShell command
   - Copy button for easy installation
   - Alternative download options

2. **Browser Extension**:
   - Download extension file from dx.ai/download
   - Enable Developer mode in browser
   - Load unpacked extension
   - Supports Chrome, Firefox, Safari, Edge, Brave

3. **IDE Extensions**:
   - Download from marketplace
   - VS Code, JetBrains, Neovim, Zed supported
   - Manual installation option

4. **Mobile/Companion Apps**:
   - App Store/Play Store links
   - Device requirements
   - Feature descriptions

### 4. Component Integration
- Replaced `PlatformIcons` with `PlatformDownloadCards` in startpage
- Updated imports in `midday/apps/website/src/components/startpage.tsx`
- Maintained same visual spacing and layout

## Technical Details

### Hover Card Implementation
- Uses shadcn-ui `HoverCard` component
- 200ms open delay for better UX
- Bottom-aligned with center positioning
- 320px width for optimal content display
- Smooth fade-in animations

### Copy Functionality
- Uses Clipboard API
- Visual feedback with check icon
- 2-second success state
- Accessible button design

### Responsive Design
- Mobile-friendly card layout
- Touch-optimized hover interactions
- Flexible grid system
- Proper spacing on all screen sizes

## Files Modified
1. `midday/apps/website/src/components/startpage.tsx` - Launch date and component integration
2. `midday/apps/website/src/components/footer.tsx` - Discord link
3. `midday/apps/website/src/app/layout.tsx` - Discord in JSON-LD schema

## Files Created
1. `midday/apps/website/src/components/platform-download-cards.tsx` - New interactive download component

## Next Steps
To see the changes:
1. Restart the development server
2. Navigate to the homepage
3. Hover over platform icons to see download details
4. Test copy functionality for curl commands
5. Verify all links work correctly

## Platform Coverage
- Desktop: macOS, Windows, Linux
- Mobile: Android, iOS
- Tablets: iPad, Android tablets
- Companion: Apple Watch, Wear OS, Apple TV, Android TV
- Web: Browser extensions (Chrome, Firefox, Safari, Edge, Brave)
- Development: IDE extensions (VS Code, JetBrains, Neovim, Zed)
- Server: VPS deployment with Docker support

## User Experience Improvements
- Clear download instructions for each platform
- One-click copy for terminal commands
- Visual feedback for all interactions
- Comprehensive platform coverage
- Consistent design language
- Accessible and keyboard-friendly
