// Load icon data from public/icons/*.json files using iconify packages
import { cacheIconPack, getCachedIconPack, getAllCachedPacks, areAllPacksCached } from './icon-cache';

export interface IconifyIcon {
  name: string;
  pack: string;
}

let memCachedIcons: IconifyIcon[] | null = null;
let memCachedPacks: string[] | null = null;

export async function getAvailableIconPacks(): Promise<string[]> {
  if (memCachedPacks) return memCachedPacks;
  
  try {
    // All 228 packs from public/icons directory
    const cachedPacks = [
      'academicons', 'akar-icons', 'ant-design', 'arcticons', 'basil', 'bi', 'bitcoin-icons',
      'bpmn', 'brandico', 'bubbles', 'bx', 'bxl', 'bxs', 'bytesize', 'carbon', 'catppuccin',
      'cbi', 'charm', 'ci', 'cib', 'cif', 'cil', 'circle-flags', 'circum', 'clarity', 'codex',
      'codicon', 'covid', 'cryptocurrency', 'cryptocurrency-color', 'cuida', 'dashicons', 'devicon',
      'devicon-line', 'devicon-original', 'devicon-plain', 'dinkie-icons', 'duo-icons', 'ei', 'el',
      'emblemicons', 'emojione', 'emojione-monotone', 'emojione-v1', 'entypo', 'entypo-social',
      'eos-icons', 'ep', 'et', 'eva', 'f7', 'fa', 'fa6-brands', 'fa6-regular', 'fa6-solid',
      'fa7-brands', 'fa7-regular', 'fa7-solid', 'fa-brands', 'fad', 'famicons', 'fa-regular',
      'fa-solid', 'fe', 'feather', 'file-icons', 'flag', 'flagpack', 'flat-color-icons', 'flat-ui',
      'flowbite', 'fluent', 'fluent-color', 'fluent-emoji', 'fluent-emoji-flat',
      'fluent-emoji-high-contrast', 'fluent-mdl2', 'fontelico', 'fontisto', 'formkit', 'foundation',
      'fxemoji', 'gala', 'game-icons', 'garden', 'geo', 'gg', 'gis', 'glyphs', 'glyphs-poly',
      'gravity-ui', 'gridicons', 'grommet-icons', 'guidance', 'healthicons', 'heroicons',
      'heroicons-outline', 'heroicons-solid', 'hugeicons', 'humbleicons', 'ic', 'icomoon-free',
      'iconamoon', 'iconoir', 'icon-park', 'icon-park-outline', 'icon-park-solid',
      'icon-park-twotone', 'icons8', 'il', 'ion', 'iwwa', 'ix', 'jam', 'la', 'lets-icons',
      'lineicons', 'line-md', 'logos', 'ls', 'lsicon', 'lucide', 'lucide-lab', 'mage',
      'majesticons', 'maki', 'map', 'marketeq', 'material-icon-theme', 'material-symbols',
      'material-symbols-light', 'mdi', 'mdi-light', 'medical-icon', 'memory', 'meteocons',
      'meteor-icons', 'mi', 'mingcute', 'mono-icons', 'mynaui', 'nimbus', 'nonicons', 'noto',
      'noto-v1', 'nrk', 'octicon', 'oi', 'ooui', 'openmoji', 'oui', 'pajamas', 'pepicons',
      'pepicons-pencil', 'pepicons-pop', 'pepicons-print', 'ph', 'picon', 'pixel', 'pixelarticons',
      'prime', 'proicons', 'ps', 'qlementine-icons', 'quill', 'radix-icons', 'raphael', 'ri',
      'rivet-icons', 'roentgen', 'si', 'sidekickicons', 'si-glyph', 'simple-icons',
      'simple-line-icons', 'skill-icons', 'solar', 'stash', 'streamline', 'streamline-block',
      'streamline-color', 'streamline-cyber', 'streamline-cyber-color', 'streamline-emojis',
      'streamline-flex', 'streamline-flex-color', 'streamline-freehand', 'streamline-freehand-color',
      'streamline-guidance', 'streamline-kameleon-color', 'streamline-logos', 'streamline-pixel',
      'streamline-plump', 'streamline-plump-color', 'streamline-sharp', 'streamline-sharp-color',
      'streamline-stickies-color', 'streamline-ultimate', 'streamline-ultimate-color', 'subway',
      'svg-spinners', 'system-uicons', 'tabler', 'tdesign', 'teenyicons', 'temaki', 'token',
      'token-branded', 'topcoat', 'twemoji', 'typcn', 'uil', 'uim', 'uis', 'uit', 'uiw', 'unjs',
      'vaadin', 'vs', 'vscode-icons', 'websymbol', 'weui', 'whh', 'wi', 'wordpress', 'wpf', 'zmdi',
      'zondicons'
    ];
    
    memCachedPacks = cachedPacks;
    return cachedPacks;
  } catch (error) {
    console.error('Failed to load icon packs:', error);
    return [];
  }
}

export async function loadAllIconData(): Promise<IconifyIcon[]> {
  if (memCachedIcons) return memCachedIcons;
  
  // Check if all packs are cached in PGlite
  const allCached = await areAllPacksCached();
  
  if (allCached) {
    const cachedPacks = await getAllCachedPacks();
    const icons: IconifyIcon[] = [];
    
    for (const pack of cachedPacks) {
      for (const iconName of pack.icons) {
        icons.push({ name: iconName, pack: pack.name });
      }
    }
    
    memCachedIcons = icons;
    return icons;
  }
  
  // Load from JSON files and cache in PGlite
  const packs = await getAvailableIconPacks();
  const icons: IconifyIcon[] = [];
  
  const batchSize = 10;
  let loadedCount = 0;
  
  for (let i = 0; i < packs.length; i += batchSize) {
    const batch = packs.slice(i, i + batchSize);
    
    await Promise.all(
      batch.map(async (pack) => {
        try {
          // Check PGlite cache first
          const cached = await getCachedIconPack(pack);
          if (cached) {
            for (const iconName of cached.icons) {
              icons.push({ name: iconName, pack });
            }
            loadedCount++;
            return;
          }
          
          // Load from JSON
          const response = await fetch(`/icons/${pack}.json`);
          if (!response.ok) {
            return;
          }
          
          const data = await response.json();
          
          if (data.icons && typeof data.icons === 'object') {
            const iconNames = Object.keys(data.icons);
            
            // Cache in PGlite
            await cacheIconPack(pack, iconNames);
            
            for (const name of iconNames) {
              icons.push({ name, pack });
            }
            loadedCount++;
          }
        } catch (error) {
          // Silent error handling
        }
      })
    );
  }
  
  memCachedIcons = icons;
  return icons;
}
