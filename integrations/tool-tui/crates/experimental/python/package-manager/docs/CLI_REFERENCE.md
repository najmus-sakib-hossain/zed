
# CLI Reference

Complete command-line reference for DX-Py package manager.

## Global Options

t:0(Option,Description)[]

## Commands

### init

Initialize a new Python project.
```bash
dx-py init [OPTIONS]
```
+-----------+-------------+
| Option    | Description |
+===========+=============+
| `--python | <VERSION>`  |
+-----------+-------------+
```bash
dx-py init dx-py init --python 3.11 dx-py init --name my-project --lib ```


### add


Add dependencies to the project.
```bash
dx-py add [OPTIONS] <PACKAGES>...
```
+---------+-------------+
| Option  | Description |
+=========+=============+
| `--dev` | Add         |
+---------+-------------+
```bash
dx-py add requests dx-py add numpy pandas matplotlib dx-py add --dev pytest black mypy dx-py add --optional docs sphinx ```

### remove

Remove dependencies from the project.
```bash
dx-py remove <PACKAGES>...
```
Examples:
```bash
dx-py remove requests dx-py remove numpy pandas ```


### lock


Generate lock file from dependencies.
```bash
dx-py lock [OPTIONS]
```
+-------------+-------------+
| Option      | Description |
+=============+=============+
| `--upgrade` | Upgrade     |
+-------------+-------------+
```bash
dx-py lock dx-py lock --upgrade dx-py lock --upgrade-package requests ```

### sync

Install packages from lock file.
```bash
dx-py sync [OPTIONS]
```
+------------+-------------+
| Option     | Description |
+============+=============+
| `--no-dev` | Skip        |
+------------+-------------+
```bash
dx-py sync dx-py sync --no-dev dx-py sync --frozen ```


### install


Lock and sync (convenience command).
```bash
dx-py install [OPTIONS]
```
Option: `--no-dev`, Description: Skip development dependencies Examples:
```bash
dx-py install dx-py install --no-dev ```

### run

Run a command in the virtual environment.
```bash
dx-py run <COMMAND> [ARGS]...
```
Examples:
```bash
dx-py run python main.py dx-py run pytest -v dx-py run black .
```

### python

Python version management.
```bash
dx-py python <SUBCOMMAND> ```
+------------+-------------+
| Subcommand | Description |
+============+=============+
| `install   | <VERSION>`  |
+------------+-------------+
```bash
dx-py python install 3.12.0 dx-py python list dx-py python pin 3.12.0 ```

### tool

Global tool management (pipx replacement).
```bash
dx-py tool <SUBCOMMAND> ```
+------------+-------------+
| Subcommand | Description |
+============+=============+
| `install   | <PACKAGE>`  |
+------------+-------------+
```bash
dx-py tool install black dx-py tool run ruff check .
dx-py tool list ```

### build

Build package for distribution.
```bash
dx-py build [OPTIONS]
```
+-----------+-------------+
| Option    | Description |
+===========+=============+
| `--wheel` | Build       |
+-----------+-------------+
```bash
dx-py build dx-py build --wheel dx-py build -o dist/ ```


### publish


Publish package to PyPI.
```bash
dx-py publish [OPTIONS]
```
+----------+-------------+
| Option   | Description |
+==========+=============+
| `--token | <TOKEN>`    |
+----------+-------------+
```bash
dx-py publish --token 10.59xYPI_TOKEN dx-py publish --repository https://test.pypi.org/legacy/ ```

### completions

Generate shell completions.
```bash
dx-py completions <SHELL> ```
+--------+--------+
| Shell  | Output |
+========+========+
| `bash` | Bash   |
+--------+--------+
```bash
dx-py completions bash > ~/.bash_completion.d/dx-py dx-py completions zsh > ~/.zfunc/_dx-py dx-py completions fish > ~/.config/fish/completions/dx-py.fish ```

## Environment Variables

+----------+-------------+
| Variable | Description |
+==========+=============+
| `DX      | PY          |
+----------+-------------+



## Exit Codes

+------+-------------+
| Code | Description |
+======+=============+
| 0    | Success     |
+------+-------------+
