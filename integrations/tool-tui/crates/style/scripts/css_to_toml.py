#!/usr/bin/env python3
"""
Script to convert CSS themes file to TOML format.
Parses @layer sections with :root and .dark selectors to create theme definitions.
"""

import re
import sys
from pathlib import Path
from typing import List, Tuple, Dict
import tomli_w

def find_matching_brace(content: str, start_pos: int) -> int:
    """Find the matching closing brace for a { at start_pos."""
    brace_count = 0
    for i in range(start_pos, len(content)):
        if content[i] == '{':
            brace_count += 1
        elif content[i] == '}':
            brace_count -= 1
            if brace_count == 0:
                return i
    return -1

def parse_css_file(css_content: str) -> List[Tuple[str, str]]:
    """Parse CSS file and extract layers."""
    layers: List[Tuple[str, str]] = []
    i = 0
    while i < len(css_content):
        # Find @layer
        layer_start = css_content.find('@layer', i)
        if layer_start == -1:
            break
        
        # Find the layer name
        name_start = layer_start + 6  # Skip @layer
        while name_start < len(css_content) and css_content[name_start].isspace():
            name_start += 1
        
        name_end = name_start
        while name_end < len(css_content) and not css_content[name_end].isspace() and css_content[name_end] != '{':
            name_end += 1
        
        layer_name = css_content[name_start:name_end].strip()
        
        # Find the opening brace
        brace_start = css_content.find('{', name_end)
        if brace_start == -1:
            break
        
        # Find the matching closing brace
        brace_end = find_matching_brace(css_content, brace_start)
        if brace_end == -1:
            break
        
        layer_content = css_content[brace_start + 1:brace_end]
        layers.append((layer_name, layer_content))
        
        i = brace_end + 1
    
    return layers

def parse_layer(layer_content: str, layer_name: str) -> Dict[str, Dict[str, str]]:
    """Parse a single @layer block."""
    theme: Dict[str, Dict[str, str]] = {}
    
    # Find :root block
    root_match = re.search(r':root\s*\{([^}]+)\}', layer_content, re.DOTALL)
    if root_match:
        root_vars: Dict[str, str] = parse_css_variables(root_match.group(1))
        theme['light'] = root_vars
    
    # Find .dark block
    dark_match = re.search(r'\.dark\s*\{([^}]+)\}', layer_content, re.DOTALL)
    if dark_match:
        dark_vars: Dict[str, str] = parse_css_variables(dark_match.group(1))
        theme['dark'] = dark_vars
    
    return theme

def parse_css_variables(css_content: str) -> Dict[str, str]:
    """Parse CSS variables from a CSS block."""
    variables: Dict[str, str] = {}
    # Match --variable: value; patterns
    var_pattern = r'--([a-zA-Z0-9-]+):\s*([^;]+);'
    matches = re.findall(var_pattern, css_content)
    for var_name, value in matches:
        # Clean up the value
        value = value.strip()
        variables[var_name] = value
    return variables

def main() -> None:
    if len(sys.argv) != 3:
        print("Usage: python css_to_toml.py <input.css> <output.toml>")
        sys.exit(1)
    
    input_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2])
    
    print(f"Input file: {input_file}")
    print(f"Output file: {output_file}")
    
    if not input_file.exists():
        print(f"Input file {input_file} does not exist")
        sys.exit(1)
    
    # Read CSS file
    css_content = input_file.read_text()
    print(f"Read {len(css_content)} characters")
    
    # Parse layers
    layers: List[Tuple[str, str]] = parse_css_file(css_content)
    print(f"Found {len(layers)} layers")
    
    themes: Dict[str, Dict[str, Dict[str, str]]] = {}
    for layer_name, layer_content in layers:
        theme = parse_layer(layer_content, layer_name)
        if theme:
            themes[layer_name] = theme
    
    print(f"Parsed {len(themes)} themes")
    
    # Convert to TOML structure - flatten to [layer.mode] format
    toml_data: Dict[str, Dict[str, str]] = {}
    for layer_name, theme in themes.items():
        for mode, variables in theme.items():
            flattened_key = f"{layer_name}.{mode}"
            toml_data[flattened_key] = variables
    
    # Write TOML file
    with open(output_file, 'wb') as f:
        tomli_w.dump(toml_data, f)
    
    print(f"Converted {len(themes)} themes to {output_file}")

if __name__ == "__main__":
    main()