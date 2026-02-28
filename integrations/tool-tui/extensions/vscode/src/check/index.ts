/**
 * DX Check Integration
 * 
 * Integrates dx-check linting into the VS Code extension.
 */

export { 
    initializeDxCheck, 
    disposeDxCheck, 
    isDxCheckRunning,
    getConfig,
    type DxCheckConfig 
} from './client';
