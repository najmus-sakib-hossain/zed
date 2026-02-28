// DX: Simplified logger without reactive-vscode
export const logger = {
  info: (...args: any[]) => console.log('[DX Explosion]', ...args),
  warn: (...args: any[]) => console.warn('[DX Explosion]', ...args),
  error: (...args: any[]) => console.error('[DX Explosion]', ...args),
}

export function isNullOrUndefined(value: any) {
  return value === null || value === undefined
}
