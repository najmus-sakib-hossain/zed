/**
 * Next.js Config Parser
 *
 * Extracts configuration values from next.config.{ts,js,mjs} files
 * using acorn AST parsing with regex fallback.
 */

import * as acorn from 'acorn';
import { stripTypescriptSyntax } from './tailwind-config-loader';

/**
 * Parse a string value from a Next.js config file.
 *
 * @param content - Raw file content of next.config.{ts,js,mjs}
 * @param key - Config key to extract (e.g., 'assetPrefix', 'basePath')
 * @param isTypeScript - Whether the content needs TypeScript syntax stripping
 * @returns The string value, or null if not found
 */
export function parseNextConfigValue(
  content: string,
  key: string,
  isTypeScript: boolean = false
): string | null {
  const processed = isTypeScript ? stripTypescriptSyntax(content) : content;
  try {
    return parseNextConfigValueAst(processed, key);
  } catch {
    return parseNextConfigValueRegex(processed, key);
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ASTNode = any;

function parseNextConfigValueAst(content: string, key: string): string | null {
  const ast = acorn.parse(content, {
    ecmaVersion: 'latest',
    sourceType: 'module',
  });

  // Collect top-level variable declarations: name -> init node
  const variables = new Map<string, ASTNode>();
  for (const node of (ast as ASTNode).body) {
    if (node.type === 'VariableDeclaration') {
      for (const decl of node.declarations) {
        if (decl.id?.name && decl.init) {
          variables.set(decl.id.name, decl.init);
        }
      }
    }
  }

  // Find the exported config object
  let configObject: ASTNode = null;

  for (const node of (ast as ASTNode).body) {
    // export default { ... } or export default configVar
    if (node.type === 'ExportDefaultDeclaration') {
      configObject = resolveToObject(node.declaration, variables);
      if (configObject) break;
    }

    // module.exports = { ... } or module.exports = configVar
    if (
      node.type === 'ExpressionStatement' &&
      node.expression.type === 'AssignmentExpression' &&
      node.expression.left.type === 'MemberExpression' &&
      node.expression.left.object?.name === 'module' &&
      node.expression.left.property?.name === 'exports'
    ) {
      configObject = resolveToObject(node.expression.right, variables);
      if (configObject) break;
    }
  }

  if (!configObject || configObject.type !== 'ObjectExpression') {
    return null;
  }

  // Find the property matching key
  for (const prop of configObject.properties) {
    if (prop.type !== 'Property') continue;

    const propName =
      prop.key.type === 'Identifier'
        ? prop.key.name
        : prop.key.type === 'Literal'
          ? String(prop.key.value)
          : null;

    if (propName !== key) continue;

    return resolveToString(prop.value, variables);
  }

  return null;
}

/** Resolve a node to an ObjectExpression, following Identifiers and CallExpressions */
function resolveToObject(
  node: ASTNode,
  variables: Map<string, ASTNode>
): ASTNode | null {
  if (!node) return null;
  if (node.type === 'ObjectExpression') return node;
  if (node.type === 'Identifier') {
    const init = variables.get(node.name);
    return init ? resolveToObject(init, variables) : null;
  }
  // Handle wrapper functions like defineConfig({ ... })
  if (node.type === 'CallExpression' && node.arguments.length > 0) {
    return resolveToObject(node.arguments[0], variables);
  }
  return null;
}

/** Resolve a node to a string value, following Identifiers */
function resolveToString(
  node: ASTNode,
  variables: Map<string, ASTNode>
): string | null {
  if (!node) return null;
  if (node.type === 'Literal' && typeof node.value === 'string') {
    return node.value;
  }
  if (node.type === 'TemplateLiteral' && node.expressions.length === 0) {
    return node.quasis[0]?.value?.cooked ?? null;
  }
  if (node.type === 'Identifier') {
    const init = variables.get(node.name);
    return init ? resolveToString(init, variables) : null;
  }
  return null;
}

function parseNextConfigValueRegex(content: string, key: string): string | null {
  const regex = new RegExp(`${key}\\s*:\\s*["'\`]([^"'\`]+)["'\`]`);
  const match = content.match(regex);
  return match ? match[1] : null;
}
