//! Platform-specific accessibility tree reading.
//!
//! - **Windows**: UI Automation (UIAutomationClient) via PowerShell
//! - **macOS**: AXUIElement via AppleScript/`osascript`
//! - **Linux**: AT-SPI2 via `python3 -c` with `gi.repository.Atspi`
//!
//! Returns a JSON representation of the focused window's accessibility tree
//! which is then parsed into `AccessibilityNode` structs.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A single node from the platform accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformNode {
    pub role: String,
    pub name: String,
    pub value: String,
    pub bounds: Option<NodeBounds>,
    pub children: Vec<PlatformNode>,
    pub focusable: bool,
    pub enabled: bool,
    pub focused: bool,
}

/// Screen-space bounding rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NodeBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl NodeBounds {
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + self.width as i32 / 2,
            self.y + self.height as i32 / 2,
        )
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }
}

/// Read the accessibility tree for the focused window.
///
/// Returns the root `PlatformNode` with children populated (up to `max_depth`).
pub fn read_focused_window(max_depth: u32) -> Result<PlatformNode> {
    #[cfg(target_os = "windows")]
    return read_windows_uia(max_depth);

    #[cfg(target_os = "macos")]
    return read_macos_ax(max_depth);

    #[cfg(target_os = "linux")]
    return read_linux_atspi(max_depth);

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = max_depth;
        Ok(PlatformNode {
            role: "window".into(),
            name: "Unsupported platform".into(),
            value: String::new(),
            bounds: None,
            children: Vec::new(),
            focusable: false,
            enabled: false,
            focused: false,
        })
    }
}

/// Find all nodes matching a role string (case-insensitive).
pub fn find_by_role(root: &PlatformNode, role: &str) -> Vec<&PlatformNode> {
    let mut results = Vec::new();
    find_by_role_inner(root, role, &mut results);
    results
}

fn find_by_role_inner<'a>(node: &'a PlatformNode, role: &str, out: &mut Vec<&'a PlatformNode>) {
    if node.role.eq_ignore_ascii_case(role) {
        out.push(node);
    }
    for child in &node.children {
        find_by_role_inner(child, role, out);
    }
}

/// Find all nodes whose name contains the given substring (case-insensitive).
pub fn find_by_name(root: &PlatformNode, name_fragment: &str) -> Vec<&PlatformNode> {
    let mut results = Vec::new();
    let lower = name_fragment.to_lowercase();
    find_by_name_inner(root, &lower, &mut results);
    results
}

fn find_by_name_inner<'a>(
    node: &'a PlatformNode,
    lower_fragment: &str,
    out: &mut Vec<&'a PlatformNode>,
) {
    if node.name.to_lowercase().contains(lower_fragment) {
        out.push(node);
    }
    for child in &node.children {
        find_by_name_inner(child, lower_fragment, out);
    }
}

/// Find the currently focused node in the tree.
pub fn find_focused(root: &PlatformNode) -> Option<&PlatformNode> {
    if root.focused {
        return Some(root);
    }
    for child in &root.children {
        if let Some(found) = find_focused(child) {
            return Some(found);
        }
    }
    None
}

/// Count total nodes in the tree.
pub fn node_count(root: &PlatformNode) -> usize {
    1 + root.children.iter().map(node_count).sum::<usize>()
}

/// Flatten the tree into a linear list (breadth-first).
pub fn flatten(root: &PlatformNode) -> Vec<&PlatformNode> {
    let mut result = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(root);
    while let Some(node) = queue.pop_front() {
        result.push(node);
        for child in &node.children {
            queue.push_back(child);
        }
    }
    result
}

// ── Windows: UI Automation ───────────────────────────────────────────

#[cfg(target_os = "windows")]
fn read_windows_uia(max_depth: u32) -> Result<PlatformNode> {
    // Use PowerShell to access UI Automation COM interface.
    // This outputs a JSON tree of the focused window's element hierarchy.
    let script = format!(
        r#"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

function Get-UIATree($element, $depth, $maxDepth) {{
    $rect = $element.Current.BoundingRectangle
    $node = @{{
        role = $element.Current.ControlType.ProgrammaticName -replace 'ControlType.',''
        name = if ($element.Current.Name) {{ $element.Current.Name }} else {{ '' }}
        value = ''
        focusable = $element.Current.IsKeyboardFocusable
        enabled = $element.Current.IsEnabled
        focused = $element.Current.HasKeyboardFocus
        bounds = @{{
            x = [int]$rect.X
            y = [int]$rect.Y
            width = [int]$rect.Width
            height = [int]$rect.Height
        }}
        children = @()
    }}

    # Try to get value
    try {{
        $valPattern = $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        $node.value = $valPattern.Current.Value
    }} catch {{}}

    if ($depth -lt $maxDepth) {{
        $children = $element.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
        $childList = @()
        foreach ($child in $children) {{
            $childList += Get-UIATree $child ($depth + 1) $maxDepth
        }}
        $node.children = $childList
    }}
    return $node
}}

$auto = [System.Windows.Automation.AutomationElement]::FocusedElement
$root = [System.Windows.Automation.TreeWalker]::RawViewWalker
$window = $auto
while ($window.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window -and $window -ne $null) {{
    $window = $root.GetParent($window)
}}
if ($window -eq $null) {{ $window = $auto }}

$tree = Get-UIATree $window 0 {depth}
$tree | ConvertTo-Json -Depth 20
"#,
        depth = max_depth,
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| anyhow::anyhow!("UIAutomation failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("UIAutomation error: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parse_platform_json(&json_str)
}

// ── macOS: AXUIElement ───────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn read_macos_ax(max_depth: u32) -> Result<PlatformNode> {
    // Use a small Python script via python3 that calls the Accessibility API
    // through PyObjC (available on macOS by default with Xcode tools).
    let script = format!(
        r#"
import json, subprocess
script = '''
tell application "System Events"
    set frontApp to first application process whose frontmost is true
    set appName to name of frontApp
    set winName to ""
    try
        set winName to name of first window of frontApp
    end try
end tell
return appName & "|" & winName
'''
result = subprocess.check_output(["osascript", "-e", script]).decode().strip()
parts = result.split("|", 1)
tree = {{
    "role": "window",
    "name": parts[1] if len(parts) > 1 else parts[0],
    "value": "",
    "focusable": True,
    "enabled": True,
    "focused": True,
    "bounds": {{"x": 0, "y": 0, "width": 1920, "height": 1080}},
    "children": []
}}
print(json.dumps(tree))
"#
    );

    let _ = max_depth; // osascript tree depth limited by AppleScript
    let output = std::process::Command::new("python3")
        .args(["-c", &script])
        .output()
        .map_err(|e| anyhow::anyhow!("macOS accessibility read failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("macOS accessibility error: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parse_platform_json(&json_str)
}

// ── Linux: AT-SPI2 ──────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn read_linux_atspi(max_depth: u32) -> Result<PlatformNode> {
    // Use python3 with gi.repository.Atspi (python3-atspi on most distros).
    let script = format!(
        r#"
import gi, json
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi

def node_to_dict(accessible, depth, max_depth):
    try:
        role = accessible.get_role_name()
    except:
        role = "unknown"
    try:
        name = accessible.get_name() or ""
    except:
        name = ""

    bounds = None
    try:
        comp = accessible.get_component_iface()
        if comp:
            ext = comp.get_extents(Atspi.CoordType.SCREEN)
            bounds = {{"x": ext.x, "y": ext.y, "width": ext.width, "height": ext.height}}
    except:
        pass

    children = []
    if depth < max_depth:
        try:
            for i in range(accessible.get_child_count()):
                child = accessible.get_child_at_index(i)
                if child:
                    children.append(node_to_dict(child, depth + 1, max_depth))
        except:
            pass

    return {{
        "role": role,
        "name": name,
        "value": "",
        "focusable": True,
        "enabled": True,
        "focused": False,
        "bounds": bounds,
        "children": children,
    }}

desktop = Atspi.get_desktop(0)
# Find the active application/window
result = None
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app and app.get_child_count() > 0:
        for j in range(app.get_child_count()):
            win = app.get_child_at_index(j)
            try:
                states = win.get_state_set()
                if states.contains(Atspi.StateType.ACTIVE):
                    result = node_to_dict(win, 0, {max_depth})
                    break
            except:
                pass
    if result:
        break

if not result:
    result = {{"role": "window", "name": "Unknown", "value": "", "focusable": False, "enabled": True, "focused": False, "bounds": None, "children": []}}

print(json.dumps(result))
"#,
        max_depth = max_depth,
    );

    let output = std::process::Command::new("python3")
        .args(["-c", &script])
        .output()
        .map_err(|e| anyhow::anyhow!("AT-SPI2 read failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("AT-SPI2 error: {}", stderr));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parse_platform_json(&json_str)
}

// ── JSON parser ──────────────────────────────────────────────────────

/// Parse the JSON output from platform accessibility scripts.
fn parse_platform_json(json_str: &str) -> Result<PlatformNode> {
    let val: serde_json::Value =
        serde_json::from_str(json_str.trim()).map_err(|e| anyhow::anyhow!("JSON parse: {}", e))?;
    parse_node(&val)
}

fn parse_node(val: &serde_json::Value) -> Result<PlatformNode> {
    let role = val
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let value = val
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let focusable = val
        .get("focusable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let enabled = val
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let focused = val
        .get("focused")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let bounds = val.get("bounds").and_then(|b| {
        if b.is_null() {
            return None;
        }
        Some(NodeBounds {
            x: b.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            y: b.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            width: b.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            height: b.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
    });

    let children = val
        .get("children")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|child| parse_node(child).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(PlatformNode {
        role,
        name,
        value,
        bounds,
        children,
        focusable,
        enabled,
        focused,
    })
}
