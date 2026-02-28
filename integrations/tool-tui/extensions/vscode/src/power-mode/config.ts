import type { WorkspaceConfiguration } from 'vscode'
import type { ThemeConfig } from './types'
import { ConfigurationTarget, workspace } from 'vscode'
import { isNullOrUndefined } from './utils'

// DX: Hardcoded config scope and defaults
const DX_CONFIG_SCOPE = 'dx'
const DX_CONFIG_DEFAULTS = {
  'explosions.enable': true,
  'explosions.maxExplosions': 10,
  'explosions.size': 10,
  'explosions.frequency': 1,
  'explosions.offset': 0.35,
  'explosions.duration': 1000,
  'explosions.customExplosions': [],
  'explosions.explosionOrder': 'sequential',
  'explosions.backgroundMode': 'image',
  'explosions.gifMode': 'restart',
  'shake.enable': false,
  'shake.intensity': 5,
  'explosion.enabled': true,
  'explosion.cyclePresets': true,
  'explosion.preset': 'fireworks',
  'explosion.comboThreshold': 0,
  'explosion.comboTimeout': 0,
  'explosion.shake.enabled': false,
}

// DX: Simplified config object without reactive-vscode
export const config = {
  $update: (key: string, value: any, target: ConfigurationTarget) => {
    workspace.getConfiguration(DX_CONFIG_SCOPE).update(key, value, target)
  },
}

type ConfigKey = string
type ThemeConfigKey = keyof ThemeConfig

function isConfigSet(key: ConfigKey, config: WorkspaceConfiguration): ConfigurationTarget | false {
  const inspectionResults = config.inspect(key)
  if (!isNullOrUndefined(inspectionResults?.workspaceFolderValue))
    return ConfigurationTarget.WorkspaceFolder
  else if (!isNullOrUndefined(inspectionResults?.workspaceValue))
    return ConfigurationTarget.Workspace
  else if (!isNullOrUndefined(inspectionResults?.globalValue))
    return ConfigurationTarget.Global
  else
    return false
}

export function getConfigValue<T>(key: ConfigKey, themeConfig?: ThemeConfig): T {
  const configuration = workspace.getConfiguration(DX_CONFIG_SCOPE)
  if (isConfigSet(key, configuration))
    return configuration.get<T>(key)!

  if (themeConfig && key in themeConfig)
    return themeConfig[key as ThemeConfigKey] as T

  return (DX_CONFIG_DEFAULTS as any)[key] as T
}
