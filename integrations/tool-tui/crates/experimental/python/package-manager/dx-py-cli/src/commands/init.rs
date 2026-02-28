//! Initialize a new Python project

use std::path::Path;

use dx_py_compat::{BuildSystem, ProjectSection, PyProjectToml};
use dx_py_core::Result;
use dx_py_workspace::{PythonManager, VenvManager};

/// Run the init command
pub fn run(path: &str, name: Option<&str>, python_version: Option<&str>) -> Result<()> {
    let project_dir = Path::new(path);

    // Create project directory if it doesn't exist
    if !project_dir.exists() {
        std::fs::create_dir_all(project_dir)?;
    }

    // Determine project name
    let project_name = name
        .map(|s| s.to_string())
        .or_else(|| project_dir.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "my-project".to_string());

    // Check if pyproject.toml already exists
    let pyproject_path = project_dir.join("pyproject.toml");
    if pyproject_path.exists() {
        println!("Project already initialized (pyproject.toml exists)");
        return Ok(());
    }

    // Find or use specified Python version
    let mut python_manager = PythonManager::new();
    python_manager.discover();

    let python_install = if let Some(version) = python_version {
        python_manager.find(version).cloned()
    } else {
        python_manager.list().first().cloned().cloned()
    };

    let requires_python = python_install
        .as_ref()
        .map(|p| {
            let parts: Vec<&str> = p.version.split('.').collect();
            if parts.len() >= 2 {
                format!(">={}.{}", parts[0], parts[1])
            } else {
                ">=3.8".to_string()
            }
        })
        .unwrap_or_else(|| ">=3.8".to_string());

    // Create pyproject.toml
    let pyproject = PyProjectToml {
        project: Some(ProjectSection {
            name: project_name.clone(),
            version: Some("0.1.0".to_string()),
            description: Some(format!("A Python project: {}", project_name)),
            requires_python: Some(requires_python),
            dependencies: Some(Vec::new()),
            ..Default::default()
        }),
        build_system: Some(BuildSystem {
            requires: vec!["hatchling".to_string()],
            build_backend: Some("hatchling.build".to_string()),
            backend_path: None,
        }),
        tool: None,
    };

    pyproject.save(&pyproject_path)?;
    println!("Created pyproject.toml");

    // Create virtual environment
    if let Some(python) = python_install {
        let venv_path = project_dir.join(".venv");
        if !venv_path.exists() {
            let mut venv_manager = VenvManager::new();
            match venv_manager.create(&venv_path, &python.path) {
                Ok(_) => println!("Created virtual environment at .venv"),
                Err(e) => println!("Warning: Could not create virtual environment: {}", e),
            }
        }
    } else {
        println!("Warning: No Python installation found. Virtual environment not created.");
        println!("Run 'dx-py python install <version>' to install Python.");
    }

    // Create source directory
    let src_dir = project_dir.join("src").join(project_name.replace('-', "_"));
    if !src_dir.exists() {
        std::fs::create_dir_all(&src_dir)?;
        std::fs::write(src_dir.join("__init__.py"), "")?;
        println!("Created src/{}/", project_name.replace('-', "_"));
    }

    // Create README.md
    let readme_path = project_dir.join("README.md");
    if !readme_path.exists() {
        std::fs::write(&readme_path, format!("# {}\n\nA Python project.\n", project_name))?;
        println!("Created README.md");
    }

    println!("\nProject '{}' initialized successfully!", project_name);
    println!("\nNext steps:");
    println!("  cd {}", path);
    println!("  dx-py add <package>  # Add dependencies");
    println!("  dx-py install        # Install dependencies");

    Ok(())
}
