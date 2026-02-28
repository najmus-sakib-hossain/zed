import type { EditorComboMeterConfig, Plugin, PowermodeChangeTextDocumentEventData } from '../types'
import * as vscode from 'vscode'

export class StatusBarComboMeter implements Plugin<EditorComboMeterConfig> {
  private config: EditorComboMeterConfig | undefined
  private statusBarItem: vscode.StatusBarItem | null = null

  dispose = () => {
    if (!this.statusBarItem)
      return
    this.statusBarItem.dispose()
    this.statusBarItem = null
  }

  public onPowermodeStart = (_combo: number) => {
    // Do nothing
  }

  public onPowermodeStop = (_combo: number) => {
    // Do nothing
  }

  public onComboStart = (combo: number) => {
    this.updateStatusBar(combo)
  }

  public onComboStop = (combo: number) => {
    this.updateStatusBar(combo)
  }

  public onDidChangeTextDocument = (data: PowermodeChangeTextDocumentEventData, _event: vscode.TextDocumentChangeEvent) => {
    this.updateStatusBar(data.currentCombo, data.isPowermodeActive)
  }

  public onDidChangeConfiguration = (config: EditorComboMeterConfig) => {
    if (this.config?.enableComboCounter === config.enableComboCounter)
      return

    this.config = config
    if (this.config.enableComboCounter)
      this.activate()
    else
      this.dispose()
  }

  private activate = () => {
    if (this.statusBarItem)
      return
    this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left)
    this.statusBarItem.show()
  }

  private updateStatusBar = (combo: number, powermode?: boolean) => {
    if (!this.statusBarItem)
      return
    const prefix = powermode ? 'POWER MODE!!! ' : ''
    this.statusBarItem.text = `${prefix}Combo: ${combo}`
  }
}
