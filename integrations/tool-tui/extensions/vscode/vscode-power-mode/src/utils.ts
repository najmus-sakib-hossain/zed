import { useLogger } from 'reactive-vscode'
import { displayName } from './generated/meta'

export const logger = useLogger(displayName)

export function isNullOrUndefined(value: any) {
  return value === null || value === undefined
}
