
# Installation Guide

## From Source

### Prerequisites

- Rust 1.70 or later
- Git

### Build Steps

```bash


# Clone repository


git clone https://github.com/example/dx-py cd dx-py/package-manager


# Build release binary


cargo build --release


# Binary location


ls target/release/dx-py ```


### Install to PATH


Linux/macOS:
```bash
sudo cp target/release/dx-py /usr/local/bin/ ```
Windows:
```powershell
Copy-Item target\release\dx-py.exe C:\Users\$env:USERNAME\.local\bin\ ```


## Shell Completions



### Bash


```bash
dx-py completions bash > ~/.bash_completion.d/dx-py source ~/.bash_completion.d/dx-py ```

### Zsh

```bash
dx-py completions zsh > ~/.zfunc/_dx-py


# Add to .zshrc: fpath=(~/.zfunc $fpath)


```

### Fish

```bash
dx-py completions fish > ~/.config/fish/completions/dx-py.fish ```


### PowerShell


```powershell
dx-py completions powershell > 10.59xROFILE.CurrentUserAllHosts ```

## Configuration

### Global Configuration

Create `~/.config/dx-py/config.toml`:
```toml
[tool.dx-py]
python_version = "3.12"
index_url = "https://pypi.org/simple"
cache_dir = "~/.cache/dx-py"
```

### Project Configuration

Add to `pyproject.toml`:
```toml
[tool.dx-py]
python_version = "3.12"
```

## Verification

```bash


# Check version


dx-py --version


# Show help


dx-py --help


# Test installation


dx-py init --python 3.12 dx-py add requests dx-py install ```


## Troubleshooting



### Build Errors


Missing OpenSSL:
```bash

# Ubuntu/Debian

sudo apt install libssl-dev pkg-config

# macOS

brew install openssl ```
Missing C compiler:
```bash


# Ubuntu/Debian


sudo apt install build-essential


# macOS


xcode-select --install ```


### Runtime Errors


Python not found:
```bash

# Install Python

dx-py python install 3.12.0 dx-py python pin 3.12.0 ```
Permission denied:
```bash


# Fix permissions


chmod +x /usr/local/bin/dx-py ```


## Uninstallation


```bash

# Remove binary

rm /usr/local/bin/dx-py

# Remove cache

rm -rf ~/.cache/dx-py

# Remove config

rm -rf ~/.config/dx-py ```
