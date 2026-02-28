import type { Plugin, ThemeConfig } from './types'
import * as vscode from 'vscode'
import { ComboPlugin } from './combo/combo-plugin'
import { config, getConfigValue } from './config'
import { CursorExploder } from './cursor-exploder'
import { THEMES } from './presets'
import { ScreenShaker } from './screen-shaker'

// DX: Cycling explosion presets
const EXPLOSION_PRESETS = ['particles', 'fireworks', 'flames', 'magic'] as const
type ExplosionPreset = typeof EXPLOSION_PRESETS[number]
let currentPresetIndex = 0

function getNextExplosionPreset(): ExplosionPreset {
  const preset = EXPLOSION_PRESETS[currentPresetIndex]
  currentPresetIndex = (currentPresetIndex + 1) % EXPLOSION_PRESETS.length
  return preset
}

function resetExplosionCycle(): void {
  currentPresetIndex = 0
}

// DX: Simplified activation without defineExtension
export function activate(context: vscode.ExtensionContext) {
  console.log('DX Explosion: Extension activating...')
  
  // Config values
  let enabled = false
  let comboThreshold: number
  let comboTimeout: number
  let comboTimeoutHandle: NodeJS.Timeout | null = null
  let cyclingEnabled = true

  // Native plugins
  let screenShaker: ScreenShaker
  let cursorExploder: CursorExploder
  let comboPlugin: ComboPlugin

  // DX Explosion components
  const plugins: Plugin[] = []

  // Current combo count
  let combo = 0
  let isDxExplosionActive = false
  let documentChangeListenerDisposer: vscode.Disposable

  // Register enable/disable commands
  vscode.commands.registerCommand('dx.explosion.enable', () => {
    console.log('DX Explosion: Enable command triggered')
    config.$update('explosion.enabled', true, vscode.ConfigurationTarget.Global)
  })
  vscode.commands.registerCommand('dx.explosion.disable', () => {
    console.log('DX Explosion: Disable command triggered')
    config.$update('explosion.enabled', false, vscode.ConfigurationTarget.Global)
  })

  const onComboTimerExpired = () => {
    console.log('DX Explosion: Combo timer expired, combo:', combo)
    plugins.forEach(plugin => plugin.onPowermodeStop(combo))
    plugins.forEach(plugin => plugin.onComboStop(combo))
    combo = 0
  }

  function isDxExplosion() {
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

  function getThemeConfig(themeId: string): ThemeConfig {
    return THEMES[themeId]
  }

  function init(config: vscode.WorkspaceConfiguration, activeTheme: ThemeConfig) {
    console.log('DX Explosion: Initializing with theme:', activeTheme)
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
    console.log('DX Explosion: Initialized, plugins count:', plugins.length)
  }

  function resetState() {
    console.log('DX Explosion: Resetting state')
    combo = 0

    stopTimer()

    documentChangeListenerDisposer?.dispose()

    while (plugins.length > 0) {
      plugins.shift()?.dispose()
    }
  }

  function onDidChangeConfiguration() {
    console.log('DX Explosion: Configuration changed')
    const config = vscode.workspace.getConfiguration('dx')
    
    // DX: Get cycling config and preset
    cyclingEnabled = getConfigValue<boolean>('explosion.cyclePresets') || false
    
    let themeId: string
    if (cyclingEnabled) {
      themeId = getNextExplosionPreset()
      console.log('DX Explosion: Cycling to next preset:', themeId)
    } else {
      themeId = getConfigValue<string>('explosion.preset') || 'fireworks'
    }
    
    const theme = getThemeConfig(themeId)

    const oldEnabled = enabled

    enabled = getConfigValue<boolean>('explosion.enabled')
    comboThreshold = getConfigValue<number>('explosion.comboThreshold') || 0
    comboTimeout = getConfigValue<number>('explosion.comboTimeout') || 0

    console.log('DX Explosion: Config - enabled:', enabled, 'cyclingEnabled:', cyclingEnabled, 'themeId:', themeId, 'comboThreshold:', comboThreshold)

    // Switching from disabled to enabled
    if (!oldEnabled && enabled) {
      console.log('DX Explosion: Enabling (was disabled)')
      resetExplosionCycle()
      init(config, theme)
      return
    }

    // Switching from enabled to disabled
    if (oldEnabled && !enabled) {
      console.log('DX Explosion: Disabling (was enabled)')
      resetState()
      return
    }

    // If not enabled, nothing matters
    // because it will be taken care of
    // when it gets reenabled
    if (!enabled) {
      console.log('DX Explosion: Not enabled, skipping config update')
      return
    }

    // The theme needs set BEFORE onDidChangeConfiguration is called
    screenShaker.themeConfig = theme
    cursorExploder.themeConfig = theme

    plugins.forEach(plugin => plugin.onDidChangeConfiguration(config))
  }

  function onDidChangeTextDocument(event: vscode.TextDocumentChangeEvent) {
    const activeEditor = vscode.window.activeTextEditor

    if (!activeEditor)
      return

    // DX: Only trigger on actual content changes, not just document events
    if (event.contentChanges.length === 0)
      return

    // DX: Ignore changes that are not user-initiated (e.g., formatting, source control)
    if (event.document !== activeEditor.document)
      return

    combo++
    const explosionActive = isDxExplosion()

    console.log('DX Explosion: Text changed, combo:', combo, 'explosionActive:', explosionActive, 'enabled:', enabled, 'threshold:', comboThreshold)

    startTimer()

    if (explosionActive !== isDxExplosionActive) {
      isDxExplosionActive = explosionActive

      console.log('DX Explosion: State changed to:', isDxExplosionActive ? 'ACTIVE' : 'INACTIVE')

      isDxExplosionActive
        ? plugins.forEach(plugin => plugin.onPowermodeStart(combo))
        : plugins.forEach(plugin => plugin.onPowermodeStop(combo))
    }

    // DX: Cycle preset on each keystroke if cycling is enabled
    if (cyclingEnabled && explosionActive) {
      const config = vscode.workspace.getConfiguration('dx')
      const themeId = getNextExplosionPreset()
      const theme = getThemeConfig(themeId)
      console.log('DX Explosion: Cycling to preset:', themeId)
      
      screenShaker.themeConfig = theme
      cursorExploder.themeConfig = theme
      plugins.forEach(plugin => plugin.onDidChangeConfiguration(config))
    }

    plugins.forEach(plugin => plugin.onDidChangeTextDocument({
      isPowermodeActive: isDxExplosionActive,
      comboTimeout,
      currentCombo: combo,
      activeEditor,
    }, event))
  }

  // Subscribe to configuration changes
  context.subscriptions.push(vscode.workspace.onDidChangeConfiguration(onDidChangeConfiguration))

  // Initialize from the current configuration
  console.log('DX Explosion: Starting initial configuration')
  onDidChangeConfiguration()
  console.log('DX Explosion: Extension activated')
}

export function deactivate() {
  console.log('DX Explosion: Extension deactivating')
}
