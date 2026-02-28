import type { WorkspaceConfiguration } from 'vscode'
import type { ThemeConfig } from './types'
import { defineConfigObject } from 'reactive-vscode'
import { ConfigurationTarget, workspace } from 'vscode'
import * as Meta from './generated/meta'
import { isNullOrUndefined } from './utils'

export const config = defineConfigObject<Meta.ScopedConfigKeyTypeMap>(
  Meta.scopedConfigs.scope,
  Meta.scopedConfigs.defaults,
)

type ConfigKey = keyof Meta.ScopedConfigKeyTypeMap
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

export function getConfigValue<T>(key: ConfigKey, themeConfig?: ThemeConfig) {
  const configuration = workspace.getConfiguration(Meta.scopedConfigs.scope)
  if (isConfigSet(key, configuration))
    return configuration.get<T>(key)!

  if (themeConfig && key in themeConfig)
    return themeConfig[key as ThemeConfigKey] as T

  return Meta.scopedConfigs.defaults[key] as T
}
