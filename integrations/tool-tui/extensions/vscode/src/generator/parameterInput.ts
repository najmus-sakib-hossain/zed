/**
 * DX Generator Parameter Input
 * 
 * Provides parameter input dialogs for template generation.
 * Requirements: 2.4
 */

import * as vscode from 'vscode';
import { TemplateMetadata, ParameterSchema } from './types';

/**
 * Parameter input handler for collecting template parameters
 */
export class ParameterInput {
    /**
     * Prompt user for missing template parameters
     */
    async promptForParameters(
        template: TemplateMetadata,
        existingParams: Record<string, string>
    ): Promise<Record<string, string> | undefined> {
        const result: Record<string, string> = { ...existingParams };

        // Get parameters that need input
        const missingParams = template.parameters.filter(
            (p) => p.required && !(p.name in existingParams)
        );

        for (const param of missingParams) {
            const value = await this.promptForSingleParameter(param);
            if (value === undefined) {
                // User cancelled
                return undefined;
            }
            result[param.name] = value;
        }

        // Optionally prompt for optional parameters
        const optionalParams = template.parameters.filter(
            (p) => !p.required && !(p.name in existingParams)
        );

        if (optionalParams.length > 0) {
            const fillOptional = await vscode.window.showQuickPick(
                ['Yes', 'No'],
                {
                    placeHolder: 'Fill optional parameters?',
                }
            );

            if (fillOptional === 'Yes') {
                for (const param of optionalParams) {
                    const value = await this.promptForSingleParameter(param);
                    if (value !== undefined && value !== '') {
                        result[param.name] = value;
                    }
                }
            }
        }

        return result;
    }

    /**
     * Prompt for a single parameter value
     */
    private async promptForSingleParameter(
        param: ParameterSchema
    ): Promise<string | undefined> {
        const defaultValue = param.default !== undefined
            ? String(param.default)
            : undefined;

        // Handle boolean type with quick pick
        if (param.valueType === 'boolean') {
            const selected = await vscode.window.showQuickPick(
                ['true', 'false'],
                {
                    placeHolder: `${param.name}: ${param.description}`,
                }
            );
            return selected;
        }

        // Handle other types with input box
        const placeholder = this.getPlaceholder(param);
        const prompt = `${param.name} (${param.valueType}): ${param.description}`;

        const value = await vscode.window.showInputBox({
            prompt,
            placeHolder: placeholder,
            value: defaultValue,
            validateInput: (input) => this.validateInput(param, input),
        });

        return value;
    }


    /**
     * Get placeholder text for a parameter
     */
    private getPlaceholder(param: ParameterSchema): string {
        if (param.examples.length > 0) {
            return `e.g., ${param.examples[0]}`;
        }

        switch (param.valueType) {
            case 'PascalCase':
                return 'e.g., MyComponent';
            case 'camelCase':
                return 'e.g., myVariable';
            case 'snake_case':
                return 'e.g., my_variable';
            case 'kebab-case':
                return 'e.g., my-component';
            case 'UPPER_CASE':
                return 'e.g., MY_CONSTANT';
            case 'integer':
                return 'e.g., 42';
            case 'float':
                return 'e.g., 3.14';
            case 'date':
                return 'e.g., 2024-01-15';
            case 'array':
                return 'e.g., item1, item2, item3';
            default:
                return `Enter ${param.name}`;
        }
    }

    /**
     * Validate input against parameter type
     */
    private validateInput(
        param: ParameterSchema,
        input: string
    ): string | undefined {
        if (!input && param.required) {
            return `${param.name} is required`;
        }

        if (!input) {
            return undefined; // Empty optional is fine
        }

        switch (param.valueType) {
            case 'PascalCase':
                if (!/^[A-Z][a-zA-Z0-9]*$/.test(input)) {
                    return 'Must be PascalCase (e.g., MyComponent)';
                }
                break;
            case 'camelCase':
                if (!/^[a-z][a-zA-Z0-9]*$/.test(input)) {
                    return 'Must be camelCase (e.g., myVariable)';
                }
                break;
            case 'snake_case':
                if (!/^[a-z][a-z0-9_]*$/.test(input)) {
                    return 'Must be snake_case (e.g., my_variable)';
                }
                break;
            case 'kebab-case':
                if (!/^[a-z][a-z0-9-]*$/.test(input)) {
                    return 'Must be kebab-case (e.g., my-component)';
                }
                break;
            case 'UPPER_CASE':
                if (!/^[A-Z][A-Z0-9_]*$/.test(input)) {
                    return 'Must be UPPER_CASE (e.g., MY_CONSTANT)';
                }
                break;
            case 'integer':
                if (!/^-?\d+$/.test(input)) {
                    return 'Must be an integer';
                }
                break;
            case 'float':
                if (!/^-?\d+(\.\d+)?$/.test(input)) {
                    return 'Must be a number';
                }
                break;
            case 'date':
                if (!/^\d{4}-\d{2}-\d{2}$/.test(input)) {
                    return 'Must be a date (YYYY-MM-DD)';
                }
                break;
        }

        return undefined;
    }

    /**
     * Show a multi-step input wizard for complex templates
     */
    async showInputWizard(
        template: TemplateMetadata
    ): Promise<Record<string, string> | undefined> {
        const result: Record<string, string> = {};
        const totalSteps = template.parameters.filter((p) => p.required).length;
        let currentStep = 0;

        for (const param of template.parameters) {
            if (!param.required) {
                continue;
            }

            currentStep++;
            const value = await this.showWizardStep(
                param,
                currentStep,
                totalSteps
            );

            if (value === undefined) {
                return undefined; // User cancelled
            }

            result[param.name] = value;
        }

        return result;
    }

    /**
     * Show a single wizard step
     */
    private async showWizardStep(
        param: ParameterSchema,
        step: number,
        totalSteps: number
    ): Promise<string | undefined> {
        const inputBox = vscode.window.createInputBox();
        inputBox.title = `Generate: ${param.name}`;
        inputBox.step = step;
        inputBox.totalSteps = totalSteps;
        inputBox.prompt = param.description;
        inputBox.placeholder = this.getPlaceholder(param);

        if (param.default !== undefined) {
            inputBox.value = String(param.default);
        }

        return new Promise((resolve) => {
            inputBox.onDidAccept(() => {
                const value = inputBox.value;
                const error = this.validateInput(param, value);
                if (error) {
                    inputBox.validationMessage = error;
                } else {
                    inputBox.hide();
                    resolve(value);
                }
            });

            inputBox.onDidHide(() => {
                inputBox.dispose();
                resolve(undefined);
            });

            inputBox.show();
        });
    }
}
