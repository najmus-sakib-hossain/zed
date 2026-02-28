
# Migration Guide

Migrate from pip, Poetry, or UV to DX-Py.

## From pip

### requirements.txt

```bash


# Convert requirements.txt to pyproject.toml


dx-py init dx-py add $(cat requirements.txt | tr '\n' ' ')
```

### requirements-dev.txt

```bash
dx-py add --dev $(cat requirements-dev.txt | tr '\n' ' ')
```

### Constraints

DX-Py uses `pyproject.toml` for all dependency management:
```toml
[project]
dependencies = [ "requests>=2.28,<3.0", "numpy>=1.24", ]
[project.optional-dependencies]
dev = ["pytest", "black"]
```

## From Poetry

### pyproject.toml

Poetry's `pyproject.toml` is mostly compatible. Key differences: Poetry:
```toml
[tool.poetry.dependencies]
python = "^3.8"
requests = "^2.28"
[tool.poetry.dev-dependencies]
pytest = "^7.0"
```
DX-Py:
```toml
[project]
requires-python = ">=3.8"
dependencies = ["requests>=2.28,<3.0"]
[project.optional-dependencies]
dev = ["pytest>=7.0,<8.0"]
[tool.dx-py]
python_version = "3.12"
```

### poetry.lock → dx-py.lock

```bash


# Generate new lock file


dx-py lock ```


### Commands


+---------+----------+
| Poetry  | DX-Py    |
+=========+==========+
| `poetry | install` |
+---------+----------+


## From UV



### uv.toml


DX-Py reads UV configuration and merges it:
```toml

# uv.toml (still works)

[pip]
index-url = "https://pypi.org/simple"

# pyproject.toml (preferred)

[tool.dx-py]
index_url = "https://pypi.org/simple"
```


### uv.lock → dx-py.lock


```bash

# Generate new lock file

dx-py lock ```

### Commands

+-----+-------+
| UV  | DX-Py |
+=====+=======+
| `uv | pip   |
+-----+-------+



## From pipenv

### Pipfile

```bash


# Convert Pipfile to pyproject.toml


dx-py init


# Add dependencies


dx-py add $(pipenv graph --json | jq -r '.[].package.key')
```

### Pipfile.lock

```bash


# Generate new lock file


dx-py lock ```


### Commands


+---------+----------+
| pipenv  | DX-Py    |
+=========+==========+
| `pipenv | install` |
+---------+----------+


## Workspace Migration



### From Lerna/npm workspaces


```toml

# pyproject.toml

[tool.dx-py.workspace]
members = ["packages/*"]
```


### From Cargo workspaces


Similar syntax:
```toml
[tool.dx-py.workspace]
members = ["crates/*"]
exclude = ["crates/internal"]
```


## CI/CD Migration



### GitHub Actions


Before (pip):
```yaml
- run: pip install
- r requirements.txt
```
After (DX-Py):
```yaml
- run: dx-py install
```


### GitLab CI


Before (Poetry):
```yaml
script:
- poetry install
- poetry run pytest
```
After (DX-Py):
```yaml
script:
- dx-py install
- dx-py run pytest
```


## Troubleshooting



### Dependency Conflicts


```bash

# Show resolution details

dx-py lock --verbose

# Upgrade conflicting package

dx-py lock --upgrade-package <package> ```

### Missing Packages

```bash


# Check if package exists


dx-py add <package> --dry-run ```


### Version Mismatches


```bash

# Pin specific version

dx-py add "package==1.2.3"
```
