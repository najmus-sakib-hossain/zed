//! IntelliJ / Fleet configuration generator.
//!
//! Generates .idea directory configuration for JetBrains IDEs.

use super::{DesktopGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// IntelliJ configuration generator.
#[derive(Debug, Default)]
pub struct IntelliJGenerator;

impl IntelliJGenerator {
    /// Create a new IntelliJ generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate run configuration XML.
    fn generate_run_config(&self, config: &WorkspaceConfig) -> Vec<(String, String)> {
        let mut configs = Vec::new();

        for task in &config.tasks.tasks {
            let xml = format!(
                r#"<component name="ProjectRunConfigurationManager">
  <configuration default="false" name="{}" type="ShConfigurationType">
    <option name="SCRIPT_TEXT" value="{} {}" />
    <option name="INDEPENDENT_SCRIPT_PATH" value="true" />
    <option name="SCRIPT_PATH" value="" />
    <option name="SCRIPT_OPTIONS" value="" />
    <option name="INDEPENDENT_SCRIPT_WORKING_DIRECTORY" value="true" />
    <option name="SCRIPT_WORKING_DIRECTORY" value="$PROJECT_DIR$" />
    <option name="INDEPENDENT_INTERPRETER_PATH" value="true" />
    <option name="INTERPRETER_PATH" value="/bin/bash" />
    <option name="INTERPRETER_OPTIONS" value="" />
    <option name="EXECUTE_IN_TERMINAL" value="true" />
    <option name="EXECUTE_SCRIPT_FILE" value="false" />
    <method v="2" />
  </configuration>
</component>"#,
                task.label,
                task.command,
                task.args.join(" ")
            );

            let filename = format!("{}.run.xml", task.label.replace(' ', "_"));
            configs.push((filename, xml));
        }

        configs
    }

    /// Generate code style XML.
    fn generate_code_style(&self, config: &WorkspaceConfig) -> String {
        format!(
            r#"<component name="ProjectCodeStyleConfiguration">
  <code_scheme name="Project" version="173">
    <Rust>
      <option name="INDENT" value="{}" />
      <option name="USE_SPACES" value="{}" />
    </Rust>
  </code_scheme>
</component>"#,
            config.editor.tab_size,
            if config.editor.insert_spaces {
                "true"
            } else {
                "false"
            }
        )
    }
}

impl DesktopGenerator for IntelliJGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Create .idea directory structure
        let idea_dir = output_dir.join(".idea");
        let run_configs_dir = idea_dir.join("runConfigurations");
        fs::create_dir_all(&run_configs_dir).map_err(|e| crate::Error::io(&run_configs_dir, e))?;

        // Generate run configurations
        for (filename, content) in self.generate_run_config(config) {
            let path = format!(".idea/runConfigurations/{}", filename);
            files.push(GeneratedFile::new(&path, content.clone()));

            let file_path = output_dir.join(&path);
            fs::write(&file_path, &content).map_err(|e| crate::Error::io(&file_path, e))?;
        }

        // Generate code style
        let code_style_dir = idea_dir.join("codeStyles");
        fs::create_dir_all(&code_style_dir).map_err(|e| crate::Error::io(&code_style_dir, e))?;

        let code_style = self.generate_code_style(config);
        files.push(GeneratedFile::new(".idea/codeStyles/Project.xml", code_style.clone()));

        let style_path = code_style_dir.join("Project.xml");
        fs::write(&style_path, &code_style).map_err(|e| crate::Error::io(&style_path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".idea").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let idea_dir = project_dir.join(".idea");
        if idea_dir.exists() {
            // Only clean dx-generated files, not user configs
            let run_configs = idea_dir.join("runConfigurations");
            if run_configs.exists() {
                fs::remove_dir_all(&run_configs).map_err(|e| crate::Error::io(&run_configs, e))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TaskConfig;

    #[test]
    fn test_generate_run_config() {
        let mut config = WorkspaceConfig::new("test");
        config.tasks = TaskConfig::dx_defaults();

        let generator = IntelliJGenerator::new();
        let configs = generator.generate_run_config(&config);

        assert!(!configs.is_empty());
        assert!(configs.iter().any(|(name, _)| name.contains("dx_build")));
    }
}
