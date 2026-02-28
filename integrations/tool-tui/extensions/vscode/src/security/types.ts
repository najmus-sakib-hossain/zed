/**
 * TypeScript types for dx-security VS Code extension integration
 * 
 * These types mirror the Rust extension API for seamless interop.
 * _Requirements: 10.4_
 */

/**
 * Security score (0-100)
 */
export type SecurityScore = number;

/**
 * Decoration type for inline highlighting
 */
export enum DecorationType {
    Critical = 'critical',
    High = 'high',
    Medium = 'medium',
    Low = 'low',
    Info = 'info',
}

/**
 * Decoration range for inline highlighting
 */
export interface DecorationRange {
    /** Start line (0-indexed) */
    startLine: number;
    /** Start column (0-indexed) */
    startColumn: number;
    /** End line (0-indexed) */
    endLine: number;
    /** End column (0-indexed) */
    endColumn: number;
    /** Decoration type/severity */
    decorationType: DecorationType;
    /** Hover message */
    message: string;
}

/**
 * Code action kind
 */
export enum CodeActionKind {
    QuickFix = 'quickfix',
    Refactor = 'refactor',
    Source = 'source',
}

/**
 * Text edit for code actions
 */
export interface TextEdit {
    /** Start line (0-indexed) */
    startLine: number;
    /** Start column (0-indexed) */
    startColumn: number;
    /** End line (0-indexed) */
    endLine: number;
    /** End column (0-indexed) */
    endColumn: number;
    /** New text to insert */
    newText: string;
}

/**
 * Quick-fix code action
 */
export interface CodeAction {
    /** Action title displayed to user */
    title: string;
    /** Kind of action */
    kind: CodeActionKind;
    /** File to modify */
    filePath: string;
    /** Text edits to apply */
    edits: TextEdit[];
    /** Whether this is the preferred action */
    isPreferred: boolean;
}

/**
 * Finding type
 */
export enum FindingType {
    Vulnerability = 'vulnerability',
    Secret = 'secret',
    RuleViolation = 'rule_violation',
    Score = 'score',
    Progress = 'progress',
    Complete = 'complete',
}

/**
 * Severity level
 */
export enum Severity {
    Info = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/**
 * Security finding
 */
export interface Finding {
    /** Type of finding */
    findingType: FindingType;
    /** Severity level */
    severity: Severity;
    /** File path where finding was detected */
    filePath: string;
    /** Line number (1-indexed) */
    lineNumber: number;
    /** Column number */
    column: number;
    /** Human-readable message */
    message: string;
    /** Optional CVE ID for vulnerabilities */
    cveId?: string;
}

/**
 * Secret finding details
 */
export interface SecretFinding {
    /** File path */
    filePath: string;
    /** Line number (1-indexed) */
    lineNumber: number;
    /** Column number */
    column: number;
    /** Secret type description */
    secretType: string;
    /** Confidence score (0.0 - 1.0) */
    confidence: number;
}

/**
 * Vulnerability finding details
 */
export interface VulnerabilityFinding {
    /** Package name */
    package: string;
    /** Package version */
    version: string;
    /** CVE ID */
    cveId: string;
    /** Severity level */
    severity: Severity;
    /** Description */
    description: string;
}

/**
 * Scan result
 */
export interface ScanResult {
    /** Calculated security score (0-100) */
    score: SecurityScore;
    /** List of detected secrets */
    secrets: SecretFinding[];
    /** List of vulnerable dependencies */
    vulnerabilities: VulnerabilityFinding[];
    /** Number of files scanned */
    filesScanned: number;
    /** Number of files skipped (unchanged) */
    filesSkipped: number;
    /** Scan duration in milliseconds */
    durationMs: number;
}

/**
 * Status bar data
 */
export interface StatusBarData {
    /** Current score */
    score: SecurityScore;
    /** Number of findings */
    findingsCount: number;
    /** Whether a scan is in progress */
    scanning: boolean;
    /** Status bar text */
    text: string;
    /** Status bar tooltip */
    tooltip: string;
    /** Status bar color (hex) */
    color?: string;
}

/**
 * Extension API interface
 */
export interface SecurityExtensionApi {
    /** Get current security score */
    getScore(): SecurityScore;
    
    /** Get findings for a specific file */
    getFileFindings(filePath: string): Finding[];
    
    /** Subscribe to finding updates */
    subscribe(callback: (finding: Finding) => void): void;
    
    /** Get decoration ranges for a file */
    getDecorations(filePath: string): DecorationRange[];
    
    /** Get code actions for a position */
    getCodeActions(filePath: string, line: number, column: number): CodeAction[];
    
    /** Trigger a scan for a specific file */
    scanFile(filePath: string): Promise<void>;
    
    /** Trigger a full workspace scan */
    scanWorkspace(workspacePath: string): Promise<ScanResult>;
    
    /** Get status bar data */
    getStatusBarData(): StatusBarData;
}

/**
 * Get decoration color for a decoration type
 */
export function getDecorationColor(type: DecorationType): string {
    switch (type) {
        case DecorationType.Critical:
            return '#ff0000';
        case DecorationType.High:
            return '#ff8c00';
        case DecorationType.Medium:
            return '#ffd700';
        case DecorationType.Low:
            return '#1e90ff';
        case DecorationType.Info:
            return '#808080';
    }
}

/**
 * Get decoration background color for a decoration type
 */
export function getDecorationBackgroundColor(type: DecorationType): string {
    switch (type) {
        case DecorationType.Critical:
            return 'rgba(255, 0, 0, 0.2)';
        case DecorationType.High:
            return 'rgba(255, 140, 0, 0.2)';
        case DecorationType.Medium:
            return 'rgba(255, 215, 0, 0.2)';
        case DecorationType.Low:
            return 'rgba(30, 144, 255, 0.2)';
        case DecorationType.Info:
            return 'rgba(128, 128, 128, 0.1)';
    }
}

/**
 * Get severity label
 */
export function getSeverityLabel(severity: Severity): string {
    switch (severity) {
        case Severity.Critical:
            return 'Critical';
        case Severity.High:
            return 'High';
        case Severity.Medium:
            return 'Medium';
        case Severity.Low:
            return 'Low';
        case Severity.Info:
            return 'Info';
    }
}

/**
 * Get status bar icon based on score
 */
export function getStatusBarIcon(score: SecurityScore): string {
    if (score >= 80) {
        return '$(shield)';
    } else if (score >= 50) {
        return '$(warning)';
    } else {
        return '$(error)';
    }
}
