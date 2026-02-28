//! Project initialization
//!
//! Provides functionality for initializing new Python projects with best practices.
//!
//! # Features
//!
//! - Create pyproject.toml with sensible defaults
//! - Support for different project templates (library, application, CLI)
//! - Generate .gitignore with Python patterns
//! - Create README.md with project description
//! - Support src-layout and flat-layout

use std::path::PathBuf;

use crate::{Error, Result};

/// Project template type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectTemplate {
    /// Library project (for distribution on PyPI)
    #[default]
    Library,
    /// Application project (standalone application)
    Application,
    /// CLI project (command-line interface tool)
    Cli,
}

impl std::str::FromStr for ProjectTemplate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "library" | "lib" => Ok(Self::Library),
            "application" | "app" => Ok(Self::Application),
            "cli" | "command" => Ok(Self::Cli),
            _ => Err(Error::Cache(format!(
                "Unknown template '{}'. Valid options: library, application, cli",
                s
            ))),
        }
    }
}

/// Project layout type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectLayout {
    /// src-layout: code in src/<package_name>/
    #[default]
    Src,
    /// flat-layout: code in <package_name>/
    Flat,
}

/// Project initialization options
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Project description
    pub description: String,
    /// Author name
    pub author: Option<String>,
    /// Author email
    pub author_email: Option<String>,
    /// Project template
    pub template: ProjectTemplate,
    /// Project layout
    pub layout: ProjectLayout,
    /// Minimum Python version
    pub python_requires: String,
    /// Create README.md
    pub create_readme: bool,
    /// Create .gitignore
    pub create_gitignore: bool,
    /// Create tests directory
    pub create_tests: bool,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            author_email: None,
            template: ProjectTemplate::default(),
            layout: ProjectLayout::default(),
            python_requires: ">=3.9".to_string(),
            create_readme: true,
            create_gitignore: true,
            create_tests: true,
        }
    }
}

/// Project initializer
pub struct ProjectInitializer {
    /// Target directory
    target_dir: PathBuf,
    /// Initialization options
    options: InitOptions,
}

impl ProjectInitializer {
    /// Create a new project initializer
    pub fn new(target_dir: PathBuf, options: InitOptions) -> Self {
        Self {
            target_dir,
            options,
        }
    }

    /// Initialize the project
    pub fn init(&self) -> Result<InitResult> {
        // Check if pyproject.toml already exists
        let pyproject_path = self.target_dir.join("pyproject.toml");
        if pyproject_path.exists() {
            return Err(Error::Cache(format!(
                "pyproject.toml already exists at {}. Use --force to overwrite.",
                self.target_dir.display()
            )));
        }

        // Create target directory if it doesn't exist
        std::fs::create_dir_all(&self.target_dir)?;

        let mut created_files = Vec::new();

        // Create pyproject.toml
        let pyproject_content = self.generate_pyproject()?;
        std::fs::write(&pyproject_path, &pyproject_content)?;
        created_files.push(pyproject_path);

        // Create package directory structure
        let package_dir = self.create_package_structure()?;
        created_files.push(package_dir);

        // Create README.md
        if self.options.create_readme {
            let readme_path = self.target_dir.join("README.md");
            let readme_content = self.generate_readme();
            std::fs::write(&readme_path, &readme_content)?;
            created_files.push(readme_path);
        }

        // Create .gitignore
        if self.options.create_gitignore {
            let gitignore_path = self.target_dir.join(".gitignore");
            let gitignore_content = self.generate_gitignore();
            std::fs::write(&gitignore_path, &gitignore_content)?;
            created_files.push(gitignore_path);
        }

        // Create tests directory
        if self.options.create_tests {
            let tests_dir = self.target_dir.join("tests");
            std::fs::create_dir_all(&tests_dir)?;

            let test_init = tests_dir.join("__init__.py");
            std::fs::write(&test_init, "")?;
            created_files.push(test_init);

            let test_file = tests_dir.join(format!("test_{}.py", self.package_name()));
            let test_content = self.generate_test_file();
            std::fs::write(&test_file, &test_content)?;
            created_files.push(test_file);
        }

        Ok(InitResult {
            project_dir: self.target_dir.clone(),
            created_files,
        })
    }

    /// Get the normalized package name (underscores instead of hyphens)
    fn package_name(&self) -> String {
        self.options.name.replace('-', "_")
    }

    /// Create the package directory structure
    fn create_package_structure(&self) -> Result<PathBuf> {
        let package_name = self.package_name();

        let package_dir = match self.options.layout {
            ProjectLayout::Src => {
                let src_dir = self.target_dir.join("src");
                std::fs::create_dir_all(&src_dir)?;
                src_dir.join(&package_name)
            }
            ProjectLayout::Flat => self.target_dir.join(&package_name),
        };

        std::fs::create_dir_all(&package_dir)?;

        // Create __init__.py
        let init_content = self.generate_init_py();
        std::fs::write(package_dir.join("__init__.py"), &init_content)?;

        // Create main module based on template
        match self.options.template {
            ProjectTemplate::Library => {
                // Library: create a simple module
                let module_content = self.generate_library_module();
                std::fs::write(package_dir.join("core.py"), &module_content)?;
            }
            ProjectTemplate::Application => {
                // Application: create main.py
                let main_content = self.generate_app_main();
                std::fs::write(package_dir.join("main.py"), &main_content)?;
            }
            ProjectTemplate::Cli => {
                // CLI: create cli.py with argparse
                let cli_content = self.generate_cli_module();
                std::fs::write(package_dir.join("cli.py"), &cli_content)?;

                let main_content = self.generate_cli_main();
                std::fs::write(package_dir.join("__main__.py"), &main_content)?;
            }
        }

        Ok(package_dir)
    }

    /// Generate pyproject.toml content
    fn generate_pyproject(&self) -> Result<String> {
        let package_name = self.package_name();

        let authors =
            if let (Some(name), Some(email)) = (&self.options.author, &self.options.author_email) {
                format!("authors = [{{ name = \"{}\", email = \"{}\" }}]\n", name, email)
            } else if let Some(name) = &self.options.author {
                format!("authors = [{{ name = \"{}\" }}]\n", name)
            } else {
                String::new()
            };

        let description = if self.options.description.is_empty() {
            format!(
                "A Python {} project",
                match self.options.template {
                    ProjectTemplate::Library => "library",
                    ProjectTemplate::Application => "application",
                    ProjectTemplate::Cli => "CLI",
                }
            )
        } else {
            self.options.description.clone()
        };

        let scripts = match self.options.template {
            ProjectTemplate::Cli => {
                format!("\n[project.scripts]\n{} = \"{}:main\"\n", self.options.name, package_name)
            }
            _ => String::new(),
        };

        let package_dir = match self.options.layout {
            ProjectLayout::Src => "src",
            ProjectLayout::Flat => ".",
        };

        Ok(format!(
            r#"[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "{name}"
version = "{version}"
description = "{description}"
readme = "README.md"
requires-python = "{python_requires}"
{authors}license = {{ text = "MIT" }}
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
]
dependencies = []

[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "pytest-cov>=4.0",
]
{scripts}
[tool.hatch.build.targets.wheel]
packages = ["{package_dir}/{package_name}"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "-v --cov={package_name}"
"#,
            name = self.options.name,
            version = self.options.version,
            description = description,
            python_requires = self.options.python_requires,
            authors = authors,
            scripts = scripts,
            package_dir = package_dir,
            package_name = package_name,
        ))
    }

    /// Generate __init__.py content
    fn generate_init_py(&self) -> String {
        format!(
            r#""""{description}"""

__version__ = "{version}"
"#,
            description = if self.options.description.is_empty() {
                format!("{} package", self.options.name)
            } else {
                self.options.description.clone()
            },
            version = self.options.version,
        )
    }

    /// Generate library module content
    fn generate_library_module(&self) -> String {
        r#""""Core functionality."""


def hello(name: str = "World") -> str:
    """Return a greeting message.
    
    Args:
        name: The name to greet.
        
    Returns:
        A greeting string.
    """
    return f"Hello, {name}!"
"#
        .to_string()
    }

    /// Generate application main.py content
    fn generate_app_main(&self) -> String {
        r#""""Main application entry point."""


def main() -> None:
    """Run the application."""
    print("Hello from the application!")


if __name__ == "__main__":
    main()
"#
        .to_string()
    }

    /// Generate CLI module content
    fn generate_cli_module(&self) -> String {
        format!(
            r#""""Command-line interface for {name}."""

import argparse
import sys


def create_parser() -> argparse.ArgumentParser:
    """Create the argument parser."""
    parser = argparse.ArgumentParser(
        prog="{name}",
        description="{description}",
    )
    parser.add_argument(
        "--version",
        action="version",
        version="%(prog)s {version}",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose output",
    )
    return parser


def main(args: list[str] | None = None) -> int:
    """Main entry point for the CLI.
    
    Args:
        args: Command-line arguments (defaults to sys.argv[1:]).
        
    Returns:
        Exit code (0 for success, non-zero for failure).
    """
    parser = create_parser()
    parsed = parser.parse_args(args)
    
    if parsed.verbose:
        print("Verbose mode enabled")
    
    print("Hello from {name}!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
"#,
            name = self.options.name,
            description = if self.options.description.is_empty() {
                format!("A CLI tool for {}", self.options.name)
            } else {
                self.options.description.clone()
            },
            version = self.options.version,
        )
    }

    /// Generate CLI __main__.py content
    fn generate_cli_main(&self) -> String {
        format!(
            r#""""Allow running as python -m {package_name}."""

from {package_name}.cli import main

if __name__ == "__main__":
    main()
"#,
            package_name = self.package_name(),
        )
    }

    /// Generate README.md content
    fn generate_readme(&self) -> String {
        let description = if self.options.description.is_empty() {
            format!(
                "A Python {} project",
                match self.options.template {
                    ProjectTemplate::Library => "library",
                    ProjectTemplate::Application => "application",
                    ProjectTemplate::Cli => "CLI tool",
                }
            )
        } else {
            self.options.description.clone()
        };

        let usage = match self.options.template {
            ProjectTemplate::Library => format!(
                r#"## Usage

```python
from {} import hello

print(hello("World"))
```"#,
                self.package_name()
            ),
            ProjectTemplate::Application => format!(
                r#"## Usage

```bash
python -m {}
```"#,
                self.package_name()
            ),
            ProjectTemplate::Cli => format!(
                r#"## Usage

```bash
{} --help
```"#,
                self.options.name
            ),
        };

        format!(
            r#"# {name}

{description}

## Installation

```bash
pip install {name}
```

{usage}

## Development

```bash
# Install development dependencies
pip install -e ".[dev]"

# Run tests
pytest
```

## License

MIT
"#,
            name = self.options.name,
            description = description,
            usage = usage,
        )
    }

    /// Generate .gitignore content
    fn generate_gitignore(&self) -> String {
        r#"# Byte-compiled / optimized / DLL files
__pycache__/
*.py[cod]
*$py.class

# C extensions
*.so

# Distribution / packaging
.Python
build/
develop-eggs/
dist/
downloads/
eggs/
.eggs/
lib/
lib64/
parts/
sdist/
var/
wheels/
*.egg-info/
.installed.cfg
*.egg

# PyInstaller
*.manifest
*.spec

# Installer logs
pip-log.txt
pip-delete-this-directory.txt

# Unit test / coverage reports
htmlcov/
.tox/
.nox/
.coverage
.coverage.*
.cache
nosetests.xml
coverage.xml
*.cover
*.py,cover
.hypothesis/
.pytest_cache/

# Translations
*.mo
*.pot

# Environments
.env
.venv
env/
venv/
ENV/
env.bak/
venv.bak/

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# mypy
.mypy_cache/
.dmypy.json
dmypy.json

# Ruff
.ruff_cache/

# pyright
pyrightconfig.json

# Local development
.python-version
"#
        .to_string()
    }

    /// Generate test file content
    fn generate_test_file(&self) -> String {
        let package_name = self.package_name();

        match self.options.template {
            ProjectTemplate::Library => format!(
                r#""""Tests for {package_name}."""

from {package_name}.core import hello


def test_hello_default():
    """Test hello with default argument."""
    assert hello() == "Hello, World!"


def test_hello_with_name():
    """Test hello with custom name."""
    assert hello("Python") == "Hello, Python!"
"#,
                package_name = package_name,
            ),
            ProjectTemplate::Application => format!(
                r#""""Tests for {package_name}."""

from {package_name}.main import main


def test_main(capsys):
    """Test main function."""
    main()
    captured = capsys.readouterr()
    assert "Hello" in captured.out
"#,
                package_name = package_name,
            ),
            ProjectTemplate::Cli => format!(
                r#""""Tests for {package_name} CLI."""

from {package_name}.cli import main, create_parser


def test_create_parser():
    """Test argument parser creation."""
    parser = create_parser()
    assert parser is not None


def test_main_returns_zero():
    """Test main returns success code."""
    result = main([])
    assert result == 0


def test_main_verbose(capsys):
    """Test verbose flag."""
    main(["--verbose"])
    captured = capsys.readouterr()
    assert "Verbose" in captured.out
"#,
                package_name = package_name,
            ),
        }
    }
}

/// Result of project initialization
#[derive(Debug)]
pub struct InitResult {
    /// Project directory
    pub project_dir: PathBuf,
    /// List of created files
    pub created_files: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_library_project() {
        let temp_dir = TempDir::new().unwrap();
        let options = InitOptions {
            name: "my-library".to_string(),
            description: "A test library".to_string(),
            template: ProjectTemplate::Library,
            ..Default::default()
        };

        let initializer = ProjectInitializer::new(temp_dir.path().to_path_buf(), options);
        let result = initializer.init().unwrap();

        assert!(result.project_dir.join("pyproject.toml").exists());
        assert!(result.project_dir.join("README.md").exists());
        assert!(result.project_dir.join(".gitignore").exists());
        assert!(result.project_dir.join("src/my_library/__init__.py").exists());
        assert!(result.project_dir.join("src/my_library/core.py").exists());
        assert!(result.project_dir.join("tests/__init__.py").exists());
    }

    #[test]
    fn test_init_cli_project() {
        let temp_dir = TempDir::new().unwrap();
        let options = InitOptions {
            name: "my-cli".to_string(),
            description: "A test CLI".to_string(),
            template: ProjectTemplate::Cli,
            ..Default::default()
        };

        let initializer = ProjectInitializer::new(temp_dir.path().to_path_buf(), options);
        let result = initializer.init().unwrap();

        assert!(result.project_dir.join("pyproject.toml").exists());
        assert!(result.project_dir.join("src/my_cli/cli.py").exists());
        assert!(result.project_dir.join("src/my_cli/__main__.py").exists());

        // Check that pyproject.toml contains scripts section
        let pyproject = std::fs::read_to_string(result.project_dir.join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("[project.scripts]"));
    }

    #[test]
    fn test_init_flat_layout() {
        let temp_dir = TempDir::new().unwrap();
        let options = InitOptions {
            name: "flat-project".to_string(),
            layout: ProjectLayout::Flat,
            ..Default::default()
        };

        let initializer = ProjectInitializer::new(temp_dir.path().to_path_buf(), options);
        let result = initializer.init().unwrap();

        // Flat layout should have package directly in project root
        assert!(result.project_dir.join("flat_project/__init__.py").exists());
        assert!(!result.project_dir.join("src").exists());
    }

    #[test]
    fn test_init_existing_project_fails() {
        let temp_dir = TempDir::new().unwrap();

        // Create existing pyproject.toml
        std::fs::write(temp_dir.path().join("pyproject.toml"), "[project]\nname = \"existing\"")
            .unwrap();

        let options = InitOptions {
            name: "new-project".to_string(),
            ..Default::default()
        };

        let initializer = ProjectInitializer::new(temp_dir.path().to_path_buf(), options);
        let result = initializer.init();

        assert!(result.is_err());
    }

    #[test]
    fn test_project_template_from_str() {
        assert_eq!("library".parse::<ProjectTemplate>().unwrap(), ProjectTemplate::Library);
        assert_eq!("lib".parse::<ProjectTemplate>().unwrap(), ProjectTemplate::Library);
        assert_eq!("application".parse::<ProjectTemplate>().unwrap(), ProjectTemplate::Application);
        assert_eq!("app".parse::<ProjectTemplate>().unwrap(), ProjectTemplate::Application);
        assert_eq!("cli".parse::<ProjectTemplate>().unwrap(), ProjectTemplate::Cli);
        assert!("invalid".parse::<ProjectTemplate>().is_err());
    }
}
