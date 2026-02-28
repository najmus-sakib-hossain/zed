
# dx-check Playground

This directory contains sample files for testing dx-check's multi-language formatting and linting capabilities.

## Directory Structure

@tree:playground[]

## Usage

### Format all files (check mode)

```bash
dx-check format playground/ ```


### Format all files (write mode)


```bash
dx-check format --write playground/ ```

### Lint all files

```bash
dx-check lint playground/ ```


### Check specific language


```bash
dx-check format playground/python/ dx-check lint playground/rust/ dx-check check playground/cpp/ ```

## Sample File Contents

Each sample file contains: -A `Stack` class/struct implementation -A `Calculator` class with operation history -A `fibonacci` function -A `is_palindrome` function -Main function demonstrating usage This provides consistent test cases across all supported languages.
