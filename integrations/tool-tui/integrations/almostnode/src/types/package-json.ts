/**
 * Type definition for package.json files
 */
export interface PackageJson {
  name?: string;
  version?: string;
  main?: string;
  module?: string;
  browser?: string | Record<string, string | false>;
  types?: string;
  exports?: Record<string, unknown> | string;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  scripts?: Record<string, string>;
  [key: string]: unknown;
}
