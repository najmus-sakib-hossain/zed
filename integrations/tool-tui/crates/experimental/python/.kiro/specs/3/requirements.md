
# Requirements Document: DX-Py Hardening & Production Readiness

## Introduction

This document specifies requirements for hardening DX-Py to be a battle-tested, production-ready Python package manager. The current implementation has functional scaffolding but lacks real-world integration, error handling, cross-platform support, and robustness needed for professional use.

## Glossary

- DX-Py: The ultra-fast Python package manager being hardened
- PyPI: Python Package Index, the official package repository
- PEP_508: Python Enhancement Proposal for dependency specification format
- PEP_440: Python Enhancement Proposal for version identification
- Wheel: Python binary package format (.whl files)
- Lock_File: DPL format file containing resolved dependency versions
- Resolver: Component that determines compatible package versions
- Cache: Content-addressable storage for downloaded packages
- Venv: Python virtual environment

## Requirements

### Requirement 1: Real PyPI Integration

User Story: As a developer, I want dx-py to download and install packages from PyPI, so that I can use it as my primary package manager.

#### Acceptance Criteria

- WHEN the lock command is executed, THE Resolver SHALL fetch real package metadata from PyPI JSON API
- WHEN resolving dependencies, THE Resolver SHALL parse PEP 508 dependency strings including extras and markers
- WHEN a package is needed, THE Cache SHALL download the wheel file from PyPI with SHA256 verification
- WHEN installing packages, THE Installer SHALL extract wheel contents to site-packages
- IF a network error occurs during download, THEN THE System SHALL retry up to 3 times with exponential backoff
- WHEN downloading multiple packages, THE System SHALL download in parallel (up to 8 concurrent downloads)

### Requirement 2: PEP 440 Version Parsing

User Story: As a developer, I want dx-py to correctly parse all Python version formats, so that version constraints work correctly.

#### Acceptance Criteria

- THE Version_Parser SHALL parse epoch versions (e.g., "1!2.0.0")
- THE Version_Parser SHALL parse pre-release versions (e.g., "1.0.0a1", "1.0.0b2", "1.0.0rc1")
- THE Version_Parser SHALL parse post-release versions (e.g., "1.0.0.post1")
- THE Version_Parser SHALL parse dev versions (e.g., "1.0.0.dev1")
- THE Version_Parser SHALL parse local versions (e.g., "1.0.0+local")
- THE Version_Parser SHALL correctly compare versions according to PEP 440 ordering
- FOR ALL valid PEP 440 version strings, parsing then formatting SHALL produce an equivalent string (round-trip property)

### Requirement 3: Environment Marker Evaluation

User Story: As a developer, I want dx-py to evaluate environment markers, so that platform-specific dependencies are handled correctly.

#### Acceptance Criteria

- THE Marker_Evaluator SHALL evaluate python_version markers (e.g., "python_version >= '3.8'")
- THE Marker_Evaluator SHALL evaluate sys_platform markers (e.g., "sys_platform == 'win32'")
- THE Marker_Evaluator SHALL evaluate platform_system markers (e.g., "platform_system == 'Windows'")
- THE Marker_Evaluator SHALL evaluate implementation_name markers (e.g., "implementation_name == 'cpython'")
- THE Marker_Evaluator SHALL evaluate extra markers (e.g., "extra == 'dev'")
- THE Marker_Evaluator SHALL support boolean operators (and, or, not) in marker expressions
- IF a marker evaluates to false, THEN THE Resolver SHALL skip that dependency

### Requirement 4: Cross-Platform Wheel Selection

User Story: As a developer, I want dx-py to select the correct wheel for my platform, so that native packages work correctly.

#### Acceptance Criteria

- THE Wheel_Selector SHALL detect the current platform (Windows, macOS, Linux)
- THE Wheel_Selector SHALL detect the current architecture (x86_64, aarch64, arm64)
- THE Wheel_Selector SHALL detect the current Python implementation (cpython, pypy)
- THE Wheel_Selector SHALL parse wheel filename tags (e.g., "cp312-cp312-manylinux_2_17_x86_64")
- THE Wheel_Selector SHALL prefer platform-specific wheels over universal wheels
- THE Wheel_Selector SHALL prefer newer manylinux tags over older ones
- IF no compatible wheel exists, THEN THE System SHALL fall back to source distribution

### Requirement 5: Robust Error Handling

User Story: As a developer, I want dx-py to provide clear error messages and recover gracefully, so that I can diagnose and fix issues.

#### Acceptance Criteria

- WHEN a package is not found on PyPI, THE System SHALL display the package name and suggest similar packages
- WHEN a version constraint cannot be satisfied, THE System SHALL display the conflicting requirements
- WHEN a network error occurs, THE System SHALL display the URL and error details
- WHEN a hash verification fails, THE System SHALL display expected vs actual hash
- WHEN a wheel is incompatible, THE System SHALL display the platform requirements
- IF an operation fails, THEN THE System SHALL clean up partial state (no corrupted cache/venv)
- THE System SHALL log detailed debug information when
- -verbose flag is used

### Requirement 6: Async/Parallel Operations

User Story: As a developer, I want dx-py to perform operations in parallel, so that installations are fast.

#### Acceptance Criteria

- THE Download_Manager SHALL download multiple packages concurrently
- THE Resolver SHALL fetch package metadata in parallel during resolution
- THE Installer SHALL install multiple packages in parallel when no dependencies exist between them
- THE System SHALL limit concurrent operations to avoid overwhelming the system
- THE System SHALL display progress for long-running operations

### Requirement 7: Real Virtual Environment Management

User Story: As a developer, I want dx-py to create and manage real virtual environments, so that I can isolate project dependencies.

#### Acceptance Criteria

- WHEN creating a venv, THE Venv_Manager SHALL create a valid Python virtual environment
- THE Venv_Manager SHALL copy or symlink the Python interpreter correctly
- THE Venv_Manager SHALL generate working activation scripts for bash, zsh, fish, and PowerShell
- THE Venv_Manager SHALL set up pip and setuptools in the venv
- WHEN the run command is executed, THE System SHALL activate the venv and run the command
- THE System SHALL support venv creation on Windows, macOS, and Linux

### Requirement 8: Real Python Version Management

User Story: As a developer, I want dx-py to download and manage Python versions, so that I don't need to install Python separately.

#### Acceptance Criteria

- THE Python_Manager SHALL download Python from python-build-standalone releases
- THE Python_Manager SHALL verify download integrity with SHA256
- THE Python_Manager SHALL extract and install Python to the managed directory
- THE Python_Manager SHALL support Windows, macOS (Intel and ARM), and Linux
- THE Python_Manager SHALL list available Python versions from the release API
- WHEN a pinned version is not installed, THE System SHALL offer to install it

### Requirement 9: Real Build and Publish

User Story: As a developer, I want dx-py to build and publish packages, so that I can distribute my code.

#### Acceptance Criteria

- THE Build_System SHALL build wheel packages from pyproject.toml
- THE Build_System SHALL build source distributions (sdist)
- THE Build_System SHALL support PEP 517 build backends (hatchling, setuptools, flit, etc.)
- THE Publish_System SHALL upload packages to PyPI using the upload API
- THE Publish_System SHALL support API token authentication
- THE Publish_System SHALL support custom repository URLs (TestPyPI, private registries)

### Requirement 10: Real Tool Management

User Story: As a developer, I want dx-py to install and run global tools, so that I can use CLI tools without polluting my project.

#### Acceptance Criteria

- THE Tool_Manager SHALL create isolated virtual environments for each tool
- THE Tool_Manager SHALL install the tool package and its dependencies
- THE Tool_Manager SHALL create wrapper scripts in a bin directory
- THE Tool_Manager SHALL add the bin directory to PATH (or instruct user how to)
- WHEN running a tool ephemerally, THE System SHALL create a temporary venv, install, run, and clean up
- THE Tool_Manager SHALL support upgrading installed tools

### Requirement 11: Configuration and Settings

User Story: As a developer, I want to configure dx-py behavior, so that I can customize it for my workflow.

#### Acceptance Criteria

- THE System SHALL read configuration from pyproject.toml [tool.dx-py] section
- THE System SHALL read global configuration from ~/.config/dx-py/config.toml
- THE System SHALL support environment variables for configuration (DX_PY_CACHE_DIR, etc.)
- THE System SHALL support configuring PyPI index URL
- THE System SHALL support configuring extra index URLs
- THE System SHALL support configuring trusted hosts for private registries

### Requirement 12: Workspace Support

User Story: As a developer, I want dx-py to support monorepo workspaces, so that I can manage multiple related packages.

#### Acceptance Criteria

- THE Workspace_Manager SHALL detect workspace configuration in pyproject.toml
- THE Workspace_Manager SHALL enumerate workspace members from glob patterns
- THE Workspace_Manager SHALL resolve dependencies across all workspace members
- THE Workspace_Manager SHALL support path dependencies between workspace members
- WHEN installing in a workspace, THE System SHALL install all workspace members in development mode

### Requirement 13: Comprehensive Testing

User Story: As a maintainer, I want comprehensive tests, so that I can be confident in the code quality.

#### Acceptance Criteria

- THE Test_Suite SHALL include integration tests that hit real PyPI
- THE Test_Suite SHALL include tests for all supported platforms (Windows, macOS, Linux)
- THE Test_Suite SHALL include property-based tests for parsers and formatters
- THE Test_Suite SHALL achieve at least 80% code coverage
- THE Test_Suite SHALL include performance regression tests

## Notes

- All network operations should have configurable timeouts
- All file operations should be atomic where possible
- The system should work offline when packages are cached
- Error messages should be actionable and user-friendly
