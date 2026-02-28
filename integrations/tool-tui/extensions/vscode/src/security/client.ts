/**
 * Security client for communicating with dx-security CLI
 * 
 * Provides methods to invoke the security scanner and parse results.
 * _Requirements: 10.4, 10.5_
 */

import * as vscode from 'vscode';
import { spawn } from 'child_process';
import {
    SecurityScore,
    Finding,
    FindingType,
    Severity,
    ScanResult,
    SecretFinding,
    VulnerabilityFinding,
    DecorationRange,
    DecorationType,
    CodeAction,
    CodeActionKind,
    SecurityExtensionApi,
    StatusBarData,
    getStatusBarIcon,
} from './types';

/**
 * Security client configuration
 */
export interface SecurityClientConfig {
    /** Path to dx CLI binary */
    cliPath: string;
    /** Working directory for scans */
    workingDirectory: string;
    /** Enable verbose output */
    verbose: boolean;
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: SecurityClientConfig = {
    cliPath: 'dx',
    workingDirectory: '.',
    verbose: false,
};

/**
 * Security client for VS Code extension
 */
export class SecurityClient implements SecurityExtensionApi {
    private config: SecurityClientConfig;
    private score: SecurityScore = 100;
    private findings: Map<string, Finding[]> = new Map();
    private secrets: Map<string, SecretFinding[]> = new Map();
    private subscribers: ((finding: Finding) => void)[] = [];
    private scanning: boolean = false;

    constructor(config: Partial<SecurityClientConfig> = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }

    /**
     * Get current security score
     */
    public getScore(): SecurityScore {
        return this.score;
    }

    /**
     * Get findings for a specific file
     */
    public getFileFindings(filePath: string): Finding[] {
        return this.findings.get(filePath) || [];
    }

    /**
     * Subscribe to finding updates
     */
    public subscribe(callback: (finding: Finding) => void): void {
        this.subscribers.push(callback);
    }

    /**
     * Get decoration ranges for a file
     */
    public getDecorations(filePath: string): DecorationRange[] {
        const decorations: DecorationRange[] = [];

        // Get secret findings
        const secrets = this.secrets.get(filePath) || [];
        for (const secret of secrets) {
            decorations.push({
                startLine: secret.lineNumber - 1, // Convert to 0-indexed
                startColumn: secret.column,
                endLine: secret.lineNumber - 1,
                endColumn: secret.column + 20, // Approximate secret length
                decorationType: DecorationType.Critical,
                message: `🔐 ${secret.secretType} detected (confidence: ${Math.round(secret.confidence * 100)}%)`,
            });
        }

        // Get other findings
        const findings = this.findings.get(filePath) || [];
        for (const finding of findings) {
            const decorationType = this.severityToDecorationType(finding.severity);
            decorations.push({
                startLine: finding.lineNumber - 1,
                startColumn: finding.column,
                endLine: finding.lineNumber - 1,
                endColumn: finding.column + 10,
                decorationType,
                message: finding.message,
            });
        }

        return decorations;
    }

    /**
     * Get code actions for a position
     */
    public getCodeActions(filePath: string, line: number, column: number): CodeAction[] {
        const actions: CodeAction[] = [];

        // Check for secrets at this position
        const secrets = this.secrets.get(filePath) || [];
        for (const secret of secrets) {
            // Line numbers in cache are 1-indexed, input is 0-indexed
            if (secret.lineNumber === line + 1) {
                // Add quick-fix to remove the secret
                actions.push({
                    title: `Remove ${secret.secretType} from code`,
                    kind: CodeActionKind.QuickFix,
                    filePath,
                    edits: [{
                        startLine: line,
                        startColumn: secret.column,
                        endLine: line,
                        endColumn: secret.column + 40,
                        newText: '"<REDACTED>"',
                    }],
                    isPreferred: false,
                });

                // Add quick-fix to use environment variable
                actions.push({
                    title: `Replace ${secret.secretType} with environment variable`,
                    kind: CodeActionKind.QuickFix,
                    filePath,
                    edits: [{
                        startLine: line,
                        startColumn: secret.column,
                        endLine: line,
                        endColumn: secret.column + 40,
                        newText: 'process.env.SECRET_KEY',
                    }],
                    isPreferred: true,
                });
            }
        }

        return actions;
    }

    /**
     * Trigger a scan for a specific file
     */
    public async scanFile(filePath: string): Promise<void> {
        // Clear existing findings for this file
        this.findings.delete(filePath);
        this.secrets.delete(filePath);

        try {
            const result = await this.runScan(filePath);
            this.processResult(result);
        } catch (error) {
            console.error('Security scan failed:', error);
            throw error;
        }
    }

    /**
     * Trigger a full workspace scan
     */
    public async scanWorkspace(workspacePath: string): Promise<ScanResult> {
        // Clear all findings
        this.findings.clear();
        this.secrets.clear();
        this.scanning = true;

        try {
            const result = await this.runScan(workspacePath);
            this.processResult(result);
            return result;
        } finally {
            this.scanning = false;
        }
    }

    /**
     * Get status bar data
     */
    public getStatusBarData(): StatusBarData {
        const findingsCount = Array.from(this.findings.values())
            .reduce((sum, arr) => sum + arr.length, 0);

        return {
            score: this.score,
            findingsCount,
            scanning: this.scanning,
            text: this.scanning 
                ? '$(sync~spin) Scanning...' 
                : `${getStatusBarIcon(this.score)} ${this.score}`,
            tooltip: this.scanning
                ? 'Security scan in progress...'
                : `Security Score: ${this.score}/100\n${findingsCount} issue(s) found\nClick to view details`,
            color: this.scanning ? undefined : this.getScoreColor(),
        };
    }

    /**
     * Run the security scan CLI command
     */
    private async runScan(path: string): Promise<ScanResult> {
        return new Promise((resolve, reject) => {
            const args = ['security', 'scan', path, '--format', 'json'];
            if (this.config.verbose) {
                args.push('--verbose');
            }

            const process = spawn(this.config.cliPath, args, {
                cwd: this.config.workingDirectory,
            });

            let stdout = '';
            let stderr = '';

            process.stdout.on('data', (data) => {
                stdout += data.toString();
            });

            process.stderr.on('data', (data) => {
                stderr += data.toString();
            });

            process.on('close', (code) => {
                if (code === 0 || code === 1) {
                    // Code 1 means security violations found, but scan succeeded
                    try {
                        const result = this.parseResult(stdout);
                        resolve(result);
                    } catch (error) {
                        reject(new Error(`Failed to parse scan result: ${error}`));
                    }
                } else {
                    reject(new Error(`Scan failed with code ${code}: ${stderr}`));
                }
            });

            process.on('error', (error) => {
                reject(error);
            });
        });
    }

    /**
     * Parse scan result from JSON output
     */
    private parseResult(output: string): ScanResult {
        try {
            const json = JSON.parse(output);
            return {
                score: json.score || 100,
                secrets: (json.secrets || []).map((s: any) => ({
                    filePath: s.file_path || s.filePath,
                    lineNumber: s.line_number || s.lineNumber,
                    column: s.column || 0,
                    secretType: s.secret_type || s.secretType,
                    confidence: s.confidence || 0.9,
                })),
                vulnerabilities: (json.vulnerabilities || []).map((v: any) => ({
                    package: v.package,
                    version: v.version,
                    cveId: v.cve_id || v.cveId,
                    severity: v.severity,
                    description: v.description,
                })),
                filesScanned: json.files_scanned || json.filesScanned || 0,
                filesSkipped: json.files_skipped || json.filesSkipped || 0,
                durationMs: json.duration_ms || json.durationMs || 0,
            };
        } catch {
            // Return default result if parsing fails
            return {
                score: 100,
                secrets: [],
                vulnerabilities: [],
                filesScanned: 0,
                filesSkipped: 0,
                durationMs: 0,
            };
        }
    }

    /**
     * Process scan result and update internal state
     */
    private processResult(result: ScanResult): void {
        this.score = result.score;

        // Cache secrets by file
        for (const secret of result.secrets) {
            const existing = this.secrets.get(secret.filePath) || [];
            existing.push(secret);
            this.secrets.set(secret.filePath, existing);

            // Notify subscribers
            const finding: Finding = {
                findingType: FindingType.Secret,
                severity: Severity.Critical,
                filePath: secret.filePath,
                lineNumber: secret.lineNumber,
                column: secret.column,
                message: `${secret.secretType} detected`,
            };
            this.notifySubscribers(finding);
        }

        // Cache vulnerabilities
        for (const vuln of result.vulnerabilities) {
            const finding: Finding = {
                findingType: FindingType.Vulnerability,
                severity: vuln.severity,
                filePath: '', // Vulnerabilities are package-level
                lineNumber: 0,
                column: 0,
                message: `${vuln.package}@${vuln.version}: ${vuln.description}`,
                cveId: vuln.cveId,
            };
            this.notifySubscribers(finding);
        }
    }

    /**
     * Notify all subscribers of a new finding
     */
    private notifySubscribers(finding: Finding): void {
        for (const callback of this.subscribers) {
            try {
                callback(finding);
            } catch (error) {
                console.error('Subscriber callback error:', error);
            }
        }
    }

    /**
     * Convert severity to decoration type
     */
    private severityToDecorationType(severity: Severity): DecorationType {
        switch (severity) {
            case Severity.Critical:
                return DecorationType.Critical;
            case Severity.High:
                return DecorationType.High;
            case Severity.Medium:
                return DecorationType.Medium;
            case Severity.Low:
                return DecorationType.Low;
            default:
                return DecorationType.Info;
        }
    }

    /**
     * Get color based on score
     */
    private getScoreColor(): string | undefined {
        if (this.score >= 80) {
            return '#00ff00';
        } else if (this.score >= 50) {
            return '#ffff00';
        } else {
            return '#ff0000';
        }
    }
}
