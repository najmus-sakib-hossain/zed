import type { TextDocumentChangeEvent, TextEditor, WorkspaceConfiguration } from 'vscode'

export interface PowermodeChangeTextDocumentEventData {
  /**
   * The current value of the user's combo
   */
  currentCombo: number
  /**
   * The number of seconds until the combo times out
   */
  comboTimeout: number
  /**
   * Whether the user has reached "Power Mode" or not
   */
  isPowermodeActive: boolean
  /**
   * The active editor at the time of the event
   */
  activeEditor: TextEditor
}

export interface Plugin<T = WorkspaceConfiguration> {
  /**
   * Called when the extension is disposed and the plugin should cleanup. Remove all decorations, clear all timers, unsubscribe from all vscode api events, etc.
   */
  dispose: () => void
  /**
   * Called when "Power Mode" starts. Power Mode starts when the combo reaches a certain threshold. Plugins can do things before this point,
   * but should avoid doing the "big, flashy" things until Power Mode activates.
   * For example, a combo meter may show plain text before Power Mode activates, then show coloful, animated text afterwards.
   * @param currentCombo The current combo value
   */
  onPowermodeStart: (currentCombo: number) => void
  /**
   * Called when "Power Mode" ends. Plugins should remove any features that they reserve for Power Mode, such as extra colors or animations.
   * @param finalCombo The combo value at the time that powermode stopped before it is reset.
   */
  onPowermodeStop: (finalCombo: number) => void
  /**
   * Called when the user's combo breaks. This often occurs at the same time as onPowermodeStop, but can also be called when the combo ends and Power Mode was not started.
   * @param finalCombo The combo value at the time that the combo stopped before it is reset.
   */
  onComboStop: (finalCombo: number) => void
  /**
   * Called when the document changed, meaning the user typed a character or did some other action to modify the content.
   * @param currentCombo The current combo value
   * @param isPowermode Whether Power Mode has started or not
   * @param event The underlying vscode.TextDocumentChangeEvent
   */
  onDidChangeTextDocument: (data: PowermodeChangeTextDocumentEventData, event: TextDocumentChangeEvent) => void
  /**
   * Called when the configuration changes. Plugins are expected to respect user configuration, and can provide their own configuration options.
   * @param powermodeConfig The Power Mode extension configuration
   */
  onDidChangeConfiguration: (powermodeConfig: T) => void
}

export interface ThemeConfig extends ExplosionConfig, ScreenShakerConfig { }

export interface ExtensionConfig extends ThemeConfig {
  enabled?: boolean
  comboThreshold?: number
  comboTimeout?: number
}

export type ExplosionOrder = 'random' | 'sequential' | number
export type BackgroundMode = 'mask' | 'image'
export type GifMode = 'continue' | 'restart'
export interface ExplosionConfig {
  'explosions.enable': boolean
  'explosions.maxExplosions': number
  'explosions.size': number
  'explosions.frequency': number
  'explosions.offset': number
  'explosions.duration': number
  'explosions.customExplosions': string[]
  'explosions.explosionOrder': ExplosionOrder
  'explosions.backgroundMode': BackgroundMode
  'explosions.gifMode': GifMode
  'explosions.customCss'?: Record<string, string>
}

export interface ScreenShakerConfig {
  'shake.enable': boolean
  'shake.intensity'?: number
}

// The types used in the configuration file
export type ComboLocationConfig = 'editor' | 'statusbar' | 'default' | 'off'
export type ComboFeatureConfig = 'show' | 'hide' | 'default'

// The types used by this plugin which converts default to an actual location
export type ComboLocation = Exclude<ComboLocationConfig, 'default'>
export interface ComboPluginConfig {
  comboLocation: ComboLocation
  enableComboTimer: boolean
  enableComboCounter: boolean
  comboCounterSize: number
  customCss: Record<string, string>
}

export type EditorComboMeterConfig = Omit<ComboPluginConfig, 'comboLocation'>
