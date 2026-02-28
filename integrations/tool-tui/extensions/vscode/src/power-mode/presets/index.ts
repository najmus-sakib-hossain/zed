import type { ThemeConfig } from '../types'

import { Fireworks } from './fireworks'
import { Flames } from './flames'
import { Magic } from './magic'
import { Particles } from './particles'
import { ExplodingRift, SimpleRift } from './rift'

export const THEMES: Record<string, ThemeConfig> = {
  'fireworks': Fireworks,
  'particles': Particles,
  'flames': Flames,
  'magic': Magic,
  'simple-rift': SimpleRift,
  'exploding-rift': ExplodingRift,
}
