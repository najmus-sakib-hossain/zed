//! Remove dependencies from the project

use std::path::Path;

use dx_py_compat::PyProjectToml;
use dx_py_core::Result;

/// Run the remove command
pub fn run(packages: &[String], dev: bool) -> Result<()> {
    let pyproject_path = Path::new("pyproject.toml");

    if !pyproject_path.exists() {
        return Err(dx_py_core::Error::Cache("No pyproject.toml found.".to_string()));
    }

    let mut pyproject = PyProjectToml::load(pyproject_path)?;

    let project = pyproject.project.as_mut().ok_or_else(|| {
        dx_py_core::Error::Cache("No [project] section in pyproject.toml".to_string())
    })?;

    for package in packages {
        let pkg_name = package.to_lowercase();

        if dev {
            // Remove from optional-dependencies.dev
            if let Some(ref mut optional_deps) = project.optional_dependencies {
                if let Some(dev_deps) = optional_deps.get_mut("dev") {
                    let before_len = dev_deps.len();
                    dev_deps.retain(|d| {
                        let dep_name =
                            d.split(['>', '<', '=', '!', '~']).next().unwrap_or(d).to_lowercase();
                        dep_name != pkg_name
                    });

                    if dev_deps.len() < before_len {
                        println!("Removed {} from [project.optional-dependencies.dev]", package);
                    } else {
                        println!("{} not found in [project.optional-dependencies.dev]", package);
                    }
                }
            }
        } else {
            // Remove from dependencies
            if let Some(ref mut deps) = project.dependencies {
                let before_len = deps.len();
                deps.retain(|d| {
                    let dep_name =
                        d.split(['>', '<', '=', '!', '~']).next().unwrap_or(d).to_lowercase();
                    dep_name != pkg_name
                });

                if deps.len() < before_len {
                    println!("Removed {} from [project.dependencies]", package);
                } else {
                    println!("{} not found in [project.dependencies]", package);
                }
            }
        }
    }

    pyproject.save(pyproject_path)?;

    println!("\nRun 'dx-py install' to update the virtual environment.");

    Ok(())
}
