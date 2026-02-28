import type { ComboPluginConfig, EditorComboMeterConfig, Plugin, PowermodeChangeTextDocumentEventData } from '../types'
import * as vscode from 'vscode'
import { getConfigValue } from '../config'

export class EditorComboMeter implements Plugin<ComboPluginConfig> {
  private disposables: vscode.Disposable[] = []
  private config?: EditorComboMeterConfig
  private comboTimerDecoration: vscode.TextEditorDecorationType | null = null
  private comboCountDecoration: vscode.TextEditorDecorationType | null = null

  private renderedRange?: vscode.Range
  private renderedComboCount?: number
  private combo: number = 0
  private isPowermodeActive: boolean = false
  private initialPowermodeCombo: number = 0

  private timerDurationInMilliseconds = 0
  private timerExpirationTimestampInMilliseconds = 0
  private readonly TIMER_UPDATE_INTERVAL = 50

  private comboTimerDecorationTimer: NodeJS.Timeout | null = null
  private comboCountAnimationTimer: NodeJS.Timeout | null = null

  private static readonly DEFAULT_CSS = {
    'position': 'absolute',
    // NOTE: This positions the element off the screen when there is horizontal scroll
    // so this feature works best when "word wrap" is enabled.
    // Using "5vw" instead did not limit the position to the viewable width.
    // 'right': '5%',
    'left': 'calc(100vw - 35rem)',
    'top': '20px',

    'font-family': 'monospace',
    'font-weight': '900',

    // NOTE: Suggestion UI will still appear on top of the combo, but that is probably a good thing
    // so the extension doesn't interfere with actual usage of the product
    'z-index': 1,
    'pointer-events': 'none',
    'text-align': 'center',
  }

  constructor() {
    this.activate()
  }

  private getTextDecorationCss(): string {
    return EditorComboMeter.objectToCssString({
      ...EditorComboMeter.DEFAULT_CSS,
      ...this.config?.customCss ?? {},
    })
  }

  private get enabled(): boolean {
    return (this.config?.enableComboCounter || this.config?.enableComboTimer) ?? false
  }

  public dispose = () => {
    this.removeDecorations()
    while (this.disposables.length) {
      this.disposables.shift()?.dispose()
    }
  }

  public onPowermodeStart = (combo: number) => {
    this.isPowermodeActive = true
    this.initialPowermodeCombo = combo
  }

  public onPowermodeStop = (_finalCombo: number) => {
    this.isPowermodeActive = false
    this.initialPowermodeCombo = 0
  }

  public onComboStop = (_finalCombo: number) => {
    this.combo = 0
    this.removeDecorations()
  }

  public onDidChangeTextDocument = (data: PowermodeChangeTextDocumentEventData, _event: vscode.TextDocumentChangeEvent) => {
    this.combo = data.currentCombo
    this.timerDurationInMilliseconds = data.comboTimeout * 1000
    this.timerExpirationTimestampInMilliseconds = new Date().getTime() + this.timerDurationInMilliseconds
    this.isPowermodeActive = data.isPowermodeActive
    this.updateDecorations(data.activeEditor)
  }

  public onDidChangeConfiguration = (config: ComboPluginConfig) => {
    const oldEnableComboCounter = this.config?.enableComboCounter
    const oldEnableComboTimer = this.config?.enableComboTimer
    const oldComboCounterSize = this.config?.comboCounterSize

    this.config = config
    if (
      this.config.enableComboCounter === oldEnableComboCounter
      && this.config.enableComboTimer === oldEnableComboTimer
      && this.config.comboCounterSize === oldComboCounterSize
    ) {
      return
    }

    this.removeDecorations()
  }

  private activate = () => {
    this.disposables.push(vscode.window.onDidChangeTextEditorVisibleRanges((e: vscode.TextEditorVisibleRangesChangeEvent) => {
      this.updateDecorations(e.textEditor)
    }))
  }

  private removeDecorations = () => {
    this.renderedComboCount = 0
    this.renderedRange = undefined

    if (this.comboCountDecoration) {
      this.comboCountDecoration.dispose()
      this.comboCountDecoration = null
    }

    if (this.comboCountAnimationTimer) {
      clearTimeout(this.comboCountAnimationTimer)
      this.comboCountAnimationTimer = null
    }

    if (this.comboTimerDecoration) {
      this.comboTimerDecoration.dispose()
      this.comboTimerDecoration = null
    }

    if (this.comboTimerDecorationTimer) {
      clearInterval(this.comboTimerDecorationTimer)
      this.comboTimerDecorationTimer = null
    }
  }

  private updateDecorations = (editor: vscode.TextEditor) => {
    if (!this.enabled)
      return

    const firstVisibleRange = editor.visibleRanges.find(range => !range.isEmpty)
    if (!firstVisibleRange || this.combo < 1) {
      this.removeDecorations()
      return
    }

    const position = firstVisibleRange.start
    const range = new vscode.Range(position, position)

    if (this.combo !== this.renderedComboCount || !this.renderedRange || !range.isEqual(this.renderedRange)) {
      this.renderedComboCount = this.combo
      this.renderedRange = range
      const ranges = [range]
      this.createComboCountDecoration(this.combo, ranges, editor)
      this.createComboTimerDecoration(ranges, editor)
    }
  }

  private getSharedStyles = (comboCount: number, frameCount = 0): { textSize: string, color: string } => {
    // Because the size and color do not start to change until Power Mode starts, we cannot use the raw "count" to calculate those values
    // or else there will be a large jump when powermode activates, so instead use the value relative to the combo at which Power Mode started.
    const powerModeCombo = this.isPowermodeActive ? comboCount - this.initialPowermodeCombo : 0

    const baseTextSize = this.config?.comboCounterSize ?? getConfigValue<number>('combo.counterSize')
    const styleCount = Math.min(powerModeCombo, 25)
    // TODO: Explain how this formula works
    const textSize = this.isPowermodeActive ? ((styleCount * baseTextSize) / 100 * 0.5 ** (frameCount * 0.2) + baseTextSize) : baseTextSize
    // Only change color in power mode
    const color = `hsl(${(100 - (this.isPowermodeActive ? powerModeCombo : 0) * 1.2)}, 100%, 45%)`

    return { textSize: `${textSize}em`, color }
  }

  private createComboTimerDecoration(ranges: vscode.Range[], editor: vscode.TextEditor | undefined = vscode.window.activeTextEditor) {
    if (this.comboTimerDecorationTimer)
      clearTimeout(this.comboTimerDecorationTimer)

    if (!this.config?.enableComboTimer)
      return

    const updateComboTimerDecoration = () => {
      const timeLeft = this.timerExpirationTimestampInMilliseconds - new Date().getTime()

      if (timeLeft <= 0) {
        if (this.comboTimerDecorationTimer)
          clearTimeout(this.comboTimerDecorationTimer)
        this.comboTimerDecoration?.dispose()
        return
      }

      const timerWidth = (timeLeft / this.timerDurationInMilliseconds) * 1.5

      const { textSize, color } = this.getSharedStyles(this.combo)

      const baseCss = EditorComboMeter.objectToCssString({
        'font-size': textSize,
        'box-shadow': `0px 0px 15px ${color}`,
      })

      const lightThemeCss = EditorComboMeter.objectToCssString({
        // Because the text is a very light color, a colored stroke is needed
        // to make it stand out sufficiently on a light theme
        border: `2px solid ${color}`,
      })

      const createComboTimerBeforeDecoration = (lightTheme?: boolean): vscode.DecorationRenderOptions => {
        return {
          before: {
            contentText: '',
            backgroundColor: 'white',
            width: `${timerWidth}em`,
            color: 'white',
            height: '8px',
            textDecoration: `none; ${this.getTextDecorationCss()} ${baseCss} ${lightTheme ? lightThemeCss : ''}`,
          },
        }
      }

      const newComboTimerDecoration = vscode.window.createTextEditorDecorationType({
        // Decorations cannot use the same pseudoelement
        ...createComboTimerBeforeDecoration(),
        rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
        light: createComboTimerBeforeDecoration(true),
      })

      editor?.setDecorations(newComboTimerDecoration, ranges)

      this.comboTimerDecoration?.dispose()

      this.comboTimerDecoration = newComboTimerDecoration
    }

    this.comboTimerDecorationTimer = setInterval(updateComboTimerDecoration, this.TIMER_UPDATE_INTERVAL)
  }

  private createComboCountDecoration = (count: number, ranges: vscode.Range[], editor: vscode.TextEditor | undefined = vscode.window.activeTextEditor) => {
    if (this.comboCountAnimationTimer)
      clearTimeout(this.comboCountAnimationTimer)

    if (!this.config?.enableComboCounter)
      return

    const animateComboCountDecoration = (frameCount: number) => {
      this.comboCountDecoration?.dispose()

      const { textSize, color } = this.getSharedStyles(count, frameCount)

      const baseCss = EditorComboMeter.objectToCssString({
        'font-size': textSize,
        'text-shadow': `0px 0px 15px ${color}`,
      })

      const lightThemeCss = EditorComboMeter.objectToCssString({
        // Because the text is a very light color, a colored stroke is needed
        // to make it stand out sufficiently on a light theme
        '-webkit-text-stroke': `2px ${color}`,
      })

      const createComboCountAfterDecoration = (lightTheme?: boolean): vscode.DecorationRenderOptions => {
        return {
          after: {
            margin: '0.5em 0 0 0',
            contentText: `${count}×`,
            color: '#FFFFFF',
            textDecoration: `none; ${this.getTextDecorationCss()} ${baseCss} ${lightTheme ? lightThemeCss : ''}`,
          },
        }
      }

      const newComboCountDecoration = vscode.window.createTextEditorDecorationType({
        // Note: Different decorations cannot use the same pseudoelement
        ...createComboCountAfterDecoration(),
        rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
        light: createComboCountAfterDecoration(true),
      })

      editor?.setDecorations(newComboCountDecoration, ranges)

      this.comboCountDecoration?.dispose()

      this.comboCountDecoration = newComboCountDecoration

      // Only animate in power mode
      if (this.isPowermodeActive && frameCount < 100) {
        this.comboCountAnimationTimer = setTimeout(() => {
          animateComboCountDecoration(frameCount + 1)
        },
        // Ease-out the animation
        20 + (0.5 * frameCount))
      }
    }

    animateComboCountDecoration(0)
  }

  private static objectToCssString(settings: Record<string, string | number>): string {
    let value = ''
    const cssString = Object.keys(settings).map((setting) => {
      value = String(settings[setting])
      if (typeof value === 'string' || typeof value === 'number')
        return `${setting}: ${value};`
      return ''
    }).join(' ')

    return cssString
  }
}
