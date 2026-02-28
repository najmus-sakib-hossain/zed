import type { EditorComboMeterConfig, Plugin, PowermodeChangeTextDocumentEventData } from '../types'
import * as vscode from 'vscode'

export class StatusBarTimer implements Plugin<EditorComboMeterConfig> {
  private config: EditorComboMeterConfig | undefined
  private secondsRemaining = 0
  private progressDisposer: (() => void) | null = null
  private timerHandle: NodeJS.Timeout | null = null
  private active: boolean | null = null

  public onDidChangeConfiguration = (config: EditorComboMeterConfig) => {
    if (this.config?.enableComboTimer === config.enableComboTimer)
      return

    this.config = config
    if (!this.config.enableComboTimer)
      this.stopTimer()
  }

  public dispose(): void {
    this.stopTimer()
  }

  public onPowermodeStart(_combo: number): void {
    // Do nothing
  }

  public onPowermodeStop(_combo: number): void {
    // Do nothing
  }

  public onComboStop(_finalCombo: number): void {
    this.stopTimer()
  }

  public onDidChangeTextDocument(data: PowermodeChangeTextDocumentEventData, _event: vscode.TextDocumentChangeEvent): void {
    if (!this.config?.enableComboTimer)
      return

    if (!this.active)
      this.startTimer(data.comboTimeout)
    else
      this.extendTimer(data.comboTimeout)
  }

  /**
   * Starts a "progress" in the bottom of the vscode window
   * which displays the time remaining for the current combo
   */
  private startTimer = (timeLimit: number) => {
    if (!this.config?.enableComboTimer)
      return

    if (timeLimit === 0)
      return

    this.stopTimer()
    this.active = true
    this.secondsRemaining = timeLimit
    vscode.window.withProgress({
      location: vscode.ProgressLocation.Window,
    }, (p) => {
      return new Promise((resolve, reject) => {
        // Storing reject will allow us to
        // cancel the progress
        this.progressDisposer = reject
        p.report({ message: this.getProgressMessage() })
        this.timerHandle = setInterval(() => {
          this.secondsRemaining--
          p.report({ message: this.getProgressMessage() })
          if (this.secondsRemaining <= 0)
            this.stopTimer()
        }, 1000)
      })
    })
  }

  private extendTimer = (timeLimit: number) => {
    this.secondsRemaining = timeLimit
  }

  /**
   * Disposes the progress and clears the timer that controls it
   */
  private stopTimer = () => {
    this.active = null
    if (this.timerHandle)
      clearInterval(this.timerHandle)
    this.timerHandle = null

    if (this.progressDisposer) {
      this.progressDisposer()
      this.progressDisposer = null
    }
  }

  /**
   * Builds a message based on how much time is left on the timer
   * @returns The progress message
   */
  private getProgressMessage = () => {
    const secondsString = Math.floor(this.secondsRemaining)
    return `Combo Timer: ${secondsString} seconds`
  }
}
