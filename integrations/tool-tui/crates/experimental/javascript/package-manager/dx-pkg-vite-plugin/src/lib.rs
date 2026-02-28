//! Vite Plugin for Dx Package Manager
//!
//! Integrates dx-package-manager with Vite build tool

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitePluginConfig {
    /// Use dx for dependency resolution
    pub use_dx_resolver: bool,
    /// Enable binary package format
    pub use_binary_packages: bool,
    /// Cache directory
    pub cache_dir: String,
}

impl Default for VitePluginConfig {
    fn default() -> Self {
        Self {
            use_dx_resolver: true,
            use_binary_packages: true,
            cache_dir: "./.dx-cache".to_string(),
        }
    }
}

/// Vite plugin implementation
pub struct DxVitePlugin {
    config: VitePluginConfig,
}

impl DxVitePlugin {
    pub fn new(config: VitePluginConfig) -> Self {
        Self { config }
    }

    /// Generate Vite plugin JavaScript
    pub fn generate_plugin_js(&self) -> String {
        format!(
            r#"
// Dx Package Manager - Vite Plugin
import {{ createRequire }} from 'module';
import {{ join }} from 'path';

export default function dxPlugin(options = {{}}) {{
  const config = {{
    useDxResolver: {},
    useBinaryPackages: {},
    cacheDir: '{}',
    ...options
  }};

  return {{
    name: 'dx-package-manager',

    configResolved(resolvedConfig) {{
      console.log('⚡ Dx Package Manager: Binary-first dependency resolution enabled');
    }},

    resolveId(source, importer) {{
      if (!config.useDxResolver) return null;

      // Let Dx resolve package locations
      if (!source.startsWith('.') && !source.startsWith('/')) {{
        const dxPath = join(process.cwd(), '.dx-store', source);
        return dxPath;
      }}

      return null;
    }},

    load(id) {{
      if (config.useBinaryPackages && id.endsWith('.dxp')) {{
        // Load binary package format
        return '/* Binary package loaded via Dx */';
      }}

      return null;
    }},

    transform(code, id) {{
      // Future: Transform for binary optimization
      return null;
    }}
  }};
}}
"#,
            self.config.use_dx_resolver, self.config.use_binary_packages, self.config.cache_dir
        )
    }

    /// Generate TypeScript definitions
    pub fn generate_types(&self) -> String {
        r#"
declare module 'dx-vite-plugin' {
  export interface DxPluginOptions {
    useDxResolver?: boolean;
    useBinaryPackages?: boolean;
    cacheDir?: string;
  }

  export default function dxPlugin(options?: DxPluginOptions): any;
}
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_generation() {
        let config = VitePluginConfig::default();
        let plugin = DxVitePlugin::new(config);

        let js = plugin.generate_plugin_js();
        assert!(js.contains("dx-package-manager"));
        assert!(js.contains("useDxResolver"));

        let types = plugin.generate_types();
        assert!(types.contains("DxPluginOptions"));
    }
}
