use crate::prompts;
use crate::prompts::PromptInteraction;
use anyhow::Result;
use owo_colors::OwoColorize;
use serde_json::{Value, json};
use std::thread::sleep;
use std::time::Duration;

pub const TOTAL_TESTS: u32 = 36;

fn start_test(title: &str) {
    let s = &*prompts::SYMBOLS;
    let _ = prompts::log::step(format!("Test: {title}"));
    eprintln!("{}", s.bar.dimmed());
}

pub fn run_test(num: u32) -> Result<(String, Value)> {
    let result = match num {
        1 => test_text_prompt(),
        2 => test_input_prompt(),
        3 => test_password_prompt(),
        4 => test_confirm_prompt(),
        5 => test_select_prompt(),
        6 => test_multiselect_prompt(),
        7 => test_autocomplete_prompt(),
        8 => test_email_prompt(),
        9 => test_phone_prompt(),
        10 => test_url_prompt(),
        11 => test_number_prompt(),
        12 => test_slider_prompt(),
        13 => test_range_slider_prompt(),
        14 => test_rating_prompt(),
        15 => test_toggle_prompt(),
        16 => test_tags_prompt(),
        17 => test_date_picker_prompt(),
        18 => test_time_picker_prompt(),
        19 => test_calendar_prompt(),
        20 => test_color_picker_prompt(),
        21 => test_color_picker_advanced_prompt(),
        22 => Ok(("emoji_picker".to_string(), json!({ "status": "disabled_temporarily" }))),
        23 => Ok(("credit_card".to_string(), json!({ "status": "disabled_temporarily" }))),
        24 => test_matrix_select_prompt(),
        25 => test_search_filter_prompt(),
        26 => test_tree_select_prompt(),
        27 => test_file_browser_prompt(),
        28 => test_json_editor_prompt(),
        29 => test_markdown_editor_prompt(),
        30 => test_code_snippet_prompt(),
        31 => test_table_editor_prompt(),
        32 => test_list_editor_prompt(),
        33 => test_kanban_prompt(),
        34 => test_wizard_prompt(),
        35 => test_progress_prompt(),
        36 => test_spinner_prompt(),
        _ => return Ok(("invalid".to_string(), json!({}))),
    }?;

    Ok(result)
}

fn test_text_prompt() -> Result<(String, Value)> {
    start_test("Text");
    let value = prompts::text("What's your name?").placeholder("John Doe").interact()?;
    Ok(("text".to_string(), json!({ "value": value })))
}

fn test_input_prompt() -> Result<(String, Value)> {
    start_test("Input");
    let value = prompts::input::input("Project name?").placeholder("dx-project").interact()?;
    Ok(("input".to_string(), json!({ "value": value })))
}

fn test_password_prompt() -> Result<(String, Value)> {
    start_test("Password");
    let value = prompts::password::password("Create password").interact()?;
    Ok(("password".to_string(), json!({ "length": value.len(), "masked": "***" })))
}

fn test_confirm_prompt() -> Result<(String, Value)> {
    start_test("Confirm");
    let value = prompts::confirm("Continue setup?").initial_value(true).interact()?;
    Ok(("confirm".to_string(), json!({ "value": value })))
}

fn test_select_prompt() -> Result<(String, Value)> {
    start_test("Select");
    let value = prompts::select("Preferred language")
        .item("rust".to_string(), "Rust", "Fast and safe")
        .item("typescript".to_string(), "TypeScript", "DX friendly")
        .item("python".to_string(), "Python", "AI scripting")
        .interact()?;
    Ok(("select".to_string(), json!({ "value": value })))
}

fn test_multiselect_prompt() -> Result<(String, Value)> {
    start_test("MultiSelect");
    let value = prompts::multiselect("Select enabled features")
        .item("lint".to_string(), "Linting", "clippy + check")
        .item("test".to_string(), "Testing", "crate tests")
        .item("fmt".to_string(), "Formatting", "cargo fmt")
        .required(true)
        .interact()?;
    Ok(("multiselect".to_string(), json!({ "values": value })))
}

fn test_autocomplete_prompt() -> Result<(String, Value)> {
    start_test("Autocomplete");
    let value = prompts::autocomplete("Choose editor")
        .item_with_description("vscode".to_string(), "Visual Studio Code", "Most used")
        .item_with_description("zed".to_string(), "Zed", "Fast Rust editor")
        .item_with_description("vim".to_string(), "Vim", "Terminal classic")
        .interact()?;
    Ok(("autocomplete".to_string(), json!({ "value": value })))
}

fn test_email_prompt() -> Result<(String, Value)> {
    start_test("Email");
    let value = prompts::email("Email address").initial_value("john@example.com").interact()?;
    Ok(("email".to_string(), json!({ "value": value })))
}

fn test_phone_prompt() -> Result<(String, Value)> {
    start_test("Phone Input");
    let value = prompts::phone_input("Phone number")
        .country_code("+1")
        .initial_value("5551234567")
        .interact()?;
    Ok(("phone_input".to_string(), json!({ "value": value })))
}

fn test_url_prompt() -> Result<(String, Value)> {
    start_test("URL Input");
    let value = prompts::url("Project URL")
        .require_https(true)
        .initial_value("https://dx.dev")
        .interact()?;
    Ok(("url".to_string(), json!({ "value": value })))
}

fn test_number_prompt() -> Result<(String, Value)> {
    start_test("Number");
    let value = prompts::number("Worker count").min(1).max(128).initial_value(8).interact()?;
    Ok(("number".to_string(), json!({ "value": value })))
}

fn test_slider_prompt() -> Result<(String, Value)> {
    start_test("Slider");
    let value = prompts::slider("CPU allocation", 0, 100).step(5).initial_value(50).interact()?;
    Ok(("slider".to_string(), json!({ "value": value })))
}

fn test_range_slider_prompt() -> Result<(String, Value)> {
    start_test("Range Slider");
    let (min, max) = prompts::range_slider("Port range", 1024, 65535)
        .initial_range(3000, 9000)
        .interact()?;
    Ok(("range_slider".to_string(), json!({ "min": min, "max": max })))
}

fn test_rating_prompt() -> Result<(String, Value)> {
    start_test("Rating");
    let value = prompts::rating("Rate DX CLI").max(5).initial_value(4).interact()?;
    Ok(("rating".to_string(), json!({ "value": value })))
}

fn test_toggle_prompt() -> Result<(String, Value)> {
    start_test("Toggle");
    let value = prompts::toggle("Enable telemetry")
        .labels("Enabled", "Disabled")
        .initial_value(false)
        .interact()?;
    Ok(("toggle".to_string(), json!({ "value": value })))
}

fn test_tags_prompt() -> Result<(String, Value)> {
    start_test("Tags");
    let value = prompts::tags("Add project tags")
        .initial_tags(vec!["rust".to_string(), "cli".to_string()])
        .placeholder("add tag")
        .interact()?;
    Ok(("tags".to_string(), json!({ "values": value })))
}

fn test_date_picker_prompt() -> Result<(String, Value)> {
    start_test("Date Picker");
    let value = prompts::date_picker("Release date").initial_date(2026, 1, 1).interact()?;
    Ok(("date_picker".to_string(), json!({ "value": value })))
}

fn test_time_picker_prompt() -> Result<(String, Value)> {
    start_test("Time Picker");
    let value = prompts::time_picker("Deployment time")
        .initial_time(9, 30, 0)
        .format_24h(true)
        .interact()?;
    Ok(("time_picker".to_string(), json!({ "value": value })))
}

fn test_calendar_prompt() -> Result<(String, Value)> {
    start_test("Calendar");
    let value = prompts::calendar("Pick calendar day").initial_date(2026, 2, 1).interact()?;
    Ok(("calendar".to_string(), json!({ "value": value })))
}

fn test_color_picker_prompt() -> Result<(String, Value)> {
    start_test("Color Picker");
    let value = prompts::color_picker::color_picker("Brand color")
        .initial_color(0, 122, 255)
        .interact()?;
    Ok(("color_picker".to_string(), json!({ "value": value })))
}

fn test_color_picker_advanced_prompt() -> Result<(String, Value)> {
    start_test("Color Picker Advanced");
    let value = prompts::color_picker_advanced("Advanced color")
        .initial_color(34, 197, 94)
        .mode(prompts::ColorMode::RGB)
        .interact()?;
    Ok(("color_picker_advanced".to_string(), json!({ "value": value })))
}

fn test_emoji_picker_prompt() -> Result<(String, Value)> {
    start_test("Emoji Picker");
    let value = prompts::emoji_picker("Choose avatar emoji").interact()?;
    Ok(("emoji_picker".to_string(), json!({ "value": value })))
}

fn test_credit_card_prompt() -> Result<(String, Value)> {
    start_test("Credit Card");
    let (number, expiry, _cvv) = prompts::credit_card("Payment method").interact()?;
    Ok((
        "credit_card".to_string(),
        json!({ "number": number, "expiry": expiry, "cvv": "***" }),
    ))
}

fn test_matrix_select_prompt() -> Result<(String, Value)> {
    start_test("Matrix Select");
    let values = prompts::matrix_select("Environment matrix")
        .row(vec![
            ("dev-linux".to_string(), "Dev/Linux".to_string()),
            ("dev-win".to_string(), "Dev/Windows".to_string()),
        ])
        .row(vec![
            ("prod-linux".to_string(), "Prod/Linux".to_string()),
            ("prod-win".to_string(), "Prod/Windows".to_string()),
        ])
        .interact()?;
    Ok(("matrix_select".to_string(), json!({ "values": values })))
}

fn test_search_filter_prompt() -> Result<(String, Value)> {
    start_test("Search Filter");
    let value = prompts::search_filter("Find integration")
        .item("telegram".to_string(), "Telegram", vec!["chat".to_string(), "bot".to_string()])
        .item(
            "discord".to_string(),
            "Discord",
            vec!["chat".to_string(), "community".to_string()],
        )
        .item("slack".to_string(), "Slack", vec!["work".to_string(), "team".to_string()])
        .filter("chat")
        .filter("bot")
        .filter("work")
        .interact()?;
    Ok(("search_filter".to_string(), json!({ "value": value })))
}

fn test_tree_select_prompt() -> Result<(String, Value)> {
    start_test("Tree Select");
    let value = prompts::tree_select("Choose module")
        .node(
            prompts::TreeNode::new("core".to_string(), "Core")
                .child(prompts::TreeNode::new("runtime".to_string(), "Runtime")),
        )
        .node(
            prompts::TreeNode::new("agent".to_string(), "Agent")
                .child(prompts::TreeNode::new("gateway".to_string(), "Gateway")),
        )
        .interact()?;
    Ok(("tree_select".to_string(), json!({ "value": value })))
}

fn test_file_browser_prompt() -> Result<(String, Value)> {
    start_test("File Browser");
    let value = prompts::file_browser("Pick a file or directory")
        .allow_directories(true)
        .interact()?;
    Ok(("file_browser".to_string(), json!({ "value": value.display().to_string() })))
}

fn test_json_editor_prompt() -> Result<(String, Value)> {
    start_test("JSON Editor");
    let value = prompts::json_editor("Edit JSON")
        .initial_json("{\"name\":\"dx\",\"mode\":\"dev\"}")
        .interact()?;
    Ok(("json_editor".to_string(), json!({ "value": value })))
}

fn test_markdown_editor_prompt() -> Result<(String, Value)> {
    start_test("Markdown Editor");
    let value = prompts::markdown_editor("Edit README section")
        .initial_content("# DX\n- Rust first\n- Native CLI")
        .interact()?;
    Ok(("markdown_editor".to_string(), json!({ "value": value })))
}

fn test_code_snippet_prompt() -> Result<(String, Value)> {
    start_test("Code Snippet");
    let value = prompts::code_snippet("Choose starter snippet")
        .snippet(prompts::CodeSnippet {
            name: "Rust Main".to_string(),
            language: "rust".to_string(),
            code: "fn main() { println!(\"hello\"); }".to_string(),
            description: "Basic Rust program".to_string(),
        })
        .snippet(prompts::CodeSnippet {
            name: "Tokio App".to_string(),
            language: "rust".to_string(),
            code: "#[tokio::main]\nasync fn main() {}".to_string(),
            description: "Async runtime entrypoint".to_string(),
        })
        .interact()?;
    Ok((
        "code_snippet".to_string(),
        json!({ "name": value.name, "language": value.language }),
    ))
}

fn test_table_editor_prompt() -> Result<(String, Value)> {
    start_test("Table Editor");
    let value = prompts::table_editor("Edit team table")
        .headers(vec!["Name".to_string(), "Role".to_string()])
        .rows(vec![
            vec!["Alice".to_string(), "Engineer".to_string()],
            vec!["Bob".to_string(), "Designer".to_string()],
        ])
        .interact()?;
    Ok(("table_editor".to_string(), json!({ "rows": value })))
}

fn test_list_editor_prompt() -> Result<(String, Value)> {
    start_test("List Editor");
    let value = prompts::list_editor("Edit package list")
        .initial_items(vec!["tokio".to_string(), "axum".to_string(), "serde".to_string()])
        .interact()?;
    Ok(("list_editor".to_string(), json!({ "items": value })))
}

fn test_kanban_prompt() -> Result<(String, Value)> {
    start_test("Kanban");
    let board = prompts::kanban("Sprint board")
        .task(
            0,
            prompts::KanbanTask {
                id: "DX-1".to_string(),
                title: "Scaffold CLI".to_string(),
                description: "Create command skeleton".to_string(),
            },
        )
        .task(
            1,
            prompts::KanbanTask {
                id: "DX-2".to_string(),
                title: "Add onboarding".to_string(),
                description: "Integrate prompt components".to_string(),
            },
        )
        .interact()?;

    let columns: Vec<Value> = board
        .into_iter()
        .map(|(name, tasks)| {
            json!({
                "column": name,
                "tasks": tasks.into_iter().map(|t| json!({"id": t.id, "title": t.title})).collect::<Vec<_>>()
            })
        })
        .collect();

    Ok(("kanban".to_string(), json!({ "columns": columns })))
}

fn test_wizard_prompt() -> Result<(String, Value)> {
    start_test("Wizard");
    let completed = prompts::wizard("Guided setup")
        .step("Profile", "Enter profile details")
        .step("Workspace", "Configure workspace")
        .step("Finish", "Confirm and save")
        .interact()?;
    Ok(("wizard".to_string(), json!({ "completed_steps": completed })))
}

fn test_progress_prompt() -> Result<(String, Value)> {
    start_test("Progress");
    let mut progress = prompts::progress::progress("Checking system", 3).width(24);
    progress.start()?;
    progress.set(1)?;
    sleep(Duration::from_millis(80));
    progress.inc(1)?;
    sleep(Duration::from_millis(80));
    progress.set_message("Finalizing checks")?;
    progress.set(3)?;
    progress.finish("System check complete")?;
    Ok(("progress".to_string(), json!({ "status": "completed" })))
}

fn test_spinner_prompt() -> Result<(String, Value)> {
    start_test("Spinner");
    let mut spinner = prompts::spinner::spinner("Preparing environment");
    spinner.start()?;
    sleep(Duration::from_millis(220));
    spinner.stop("Environment ready")?;
    Ok(("spinner".to_string(), json!({ "status": "completed" })))
}
