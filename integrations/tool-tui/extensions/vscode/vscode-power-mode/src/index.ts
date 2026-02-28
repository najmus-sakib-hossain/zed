import type { Plugin, ThemeConfig } from './types'
import { defineExtension } from 'reactive-vscode'
import * as vscode from 'vscode'
import { ComboPlugin } from './combo/combo-plugin'
import { config, getConfigValue } from './config'
import { CursorExploder } from './cursor-exploder'
import * as Meta from './generated/meta'
import { THEMES } from './presets'
import { ScreenShaker } from './screen-shaker'

const { activate, deactivate } = defineExtension((context: vscode.ExtensionContext) => {
  // Config values
  let enabled = false
  let comboThreshold: number
  let comboTimeout: number
  let comboTimeoutHandle: NodeJS.Timeout | null = null

  // Native plugins
  let screenShaker: ScreenShaker
  let cursorExploder: CursorExploder
  let comboPlugin: ComboPlugin

  // PowerMode components
  const plugins: Plugin[] = []

  // Current combo count
  let combo = 0
  let isPowermodeActive = false
  let documentChangeListenerDisposer: vscode.Disposable

  // Register enable/disable commands
  vscode.commands.registerCommand('powermode.enablePowerMode', () => config.$update('enabled', true, vscode.ConfigurationTarget.Global))
  vscode.commands.registerCommand('powermode.disablePowerMode', () => config.$update('enabled', false, vscode.ConfigurationTarget.Global))

  const onComboTimerExpired = () => {
    plugins.forEach(plugin => plugin.onPowermodeStop(combo))

    plugins.forEach(plugin => plugin.onComboStop(combo))

    combo = 0
  }

  function isPowerMode() {
    return enabled && combo >= comboThreshold
  }

  /**
   * Starts a "progress" in the bottom of the vscode window
   * which displays the time remaining for the current combo
   */
  function startTimer() {
    stopTimer()

    if (comboTimeout === 0)
      return

    comboTimeoutHandle = setTimeout(onComboTimerExpired, comboTimeout * 1000)
  }

  function stopTimer() {
    if (comboTimeoutHandle)
      clearInterval(comboTimeoutHandle)
    comboTimeoutHandle = null
  }

  // This will be exposed so other extensions can contribute their own themes
  // function registerTheme(themeId: string, config: ThemeConfig) {
  //     THEMES[themeId] = config;
  // }

  function getThemeConfig(themeId: string): ThemeConfig {
    return THEMES[themeId]
  }

  function init(config: vscode.WorkspaceConfiguration, activeTheme: ThemeConfig) {
    // Just in case something was left behind, clean it up
    resetState()

    // The native plugins need this special theme, a subset of the config
    screenShaker = new ScreenShaker(activeTheme)
    cursorExploder = new CursorExploder(activeTheme)
    comboPlugin = new ComboPlugin()

    plugins.push(
      screenShaker,
      cursorExploder,
      comboPlugin,
    )

    plugins.forEach(plugin => plugin.onDidChangeConfiguration(config))

    documentChangeListenerDisposer = vscode.workspace.onDidChangeTextDocument(onDidChangeTextDocument)
  }

  function resetState() {
    combo = 0

    stopTimer()

    documentChangeListenerDisposer?.dispose()

    while (plugins.length > 0) {
      plugins.shift()?.dispose()
    }
  }

  function onDidChangeConfiguration() {
    const config = vscode.workspace.getConfiguration(Meta.scopedConfigs.scope)
    const themeId = getConfigValue<string>('presets')
    const theme = getThemeConfig(themeId)

    const oldEnabled = enabled

    enabled = getConfigValue<boolean>('enabled')
    comboThreshold = getConfigValue<number>('combo.threshold')
    comboTimeout = getConfigValue<number>('combo.timeout')

    // Switching from disabled to enabled
    if (!oldEnabled && enabled) {
      init(config, theme)
      return
    }

    // Switching from enabled to disabled
    if (oldEnabled && !enabled) {
      resetState()
      return
    }

    // If not enabled, nothing matters
    // because it will be taken care of
    // when it gets reenabled
    if (!enabled)
      return

    // The theme needs set BEFORE onDidChangeConfiguration is called
    screenShaker.themeConfig = theme
    cursorExploder.themeConfig = theme

    plugins.forEach(plugin => plugin.onDidChangeConfiguration(config))
  }

  function onDidChangeTextDocument(event: vscode.TextDocumentChangeEvent) {
    const activeEditor = vscode.window.activeTextEditor

    if (!activeEditor)
      return

    combo++
    const powermode = isPowerMode()

    startTimer()

    if (powermode !== isPowermodeActive) {
      isPowermodeActive = powermode

      isPowermodeActive
        ? plugins.forEach(plugin => plugin.onPowermodeStart(combo))
        : plugins.forEach(plugin => plugin.onPowermodeStop(combo))
    }

    plugins.forEach(plugin => plugin.onDidChangeTextDocument({
      isPowermodeActive,
      comboTimeout,
      currentCombo: combo,
      activeEditor,
    }, event))
  }

  // Subscribe to configuration changes
  context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(onDidChangeConfiguration))

  // Initialize from the current configuration
  onDidChangeConfiguration()
})

export { activate, deactivate }
