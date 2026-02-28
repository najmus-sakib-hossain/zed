use dx_www_binary::serializer::HtipWriter;
use ed25519_dalek::SigningKey;

/// Generate HTIP binary for counter demo
pub fn generate_counter_htip() -> Vec<u8> {
    let mut writer = HtipWriter::new();

    // Template 0: Counter container
    writer.write_template(
        0,
        r#"<div class="counter-app">
            <h2>DX-WWW Counter Demo</h2>
            <div class="counter-display" id="display">0</div>
            <div class="button-group">
                <button id="inc">Increment</button>
                <button id="dec">Decrement</button>
                <button id="reset">Reset</button>
            </div>
        </div>"#,
        vec![],
    );

    // Template 1: Status panel
    writer.write_template(
        1,
        r#"<div class="status-panel">
            <div class="status-item">
                <strong>Runtime:</strong> <span id="runtime">dx-www-client</span>
            </div>
            <div class="status-item">
                <strong>WASM Size:</strong> <span id="wasm-size">1.5 KB</span>
            </div>
            <div class="status-item">
                <strong>Templates:</strong> <span id="template-count">0</span>
            </div>
            <div class="status-item">
                <strong>Nodes:</strong> <span id="node-count">0</span>
            </div>
        </div>"#,
        vec![],
    );

    // Instantiate templates
    writer.write_instantiate(1, 0, 0); // Counter app
    writer.write_instantiate(2, 1, 0); // Status panel

    // Attach event handlers
    writer.write_attach_event(1, "click", 100); // Increment button
    writer.write_attach_event(1, "click", 101); // Decrement button
    writer.write_attach_event(1, "click", 102); // Reset button

    // Sign and finish
    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    writer.finish_and_sign(&signing_key).unwrap()
}

/// Generate HTIP binary for todo list demo
pub fn generate_todo_htip() -> Vec<u8> {
    let mut writer = HtipWriter::new();

    // Template 0: Todo item
    writer.write_template(
        0,
        r#"<div class="todo-item">
            <input type="checkbox" class="todo-checkbox">
            <span class="todo-text"></span>
            <button class="todo-delete">Delete</button>
        </div>"#,
        vec![],
    );

    // Template 1: Todo app container
    writer.write_template(
        1,
        r#"<div class="todo-app">
            <h2>DX-WWW Todo List</h2>
            <div class="todo-input-group">
                <input type="text" id="todo-input" placeholder="Add a new task...">
                <button id="add-btn">Add</button>
            </div>
            <div id="todo-list"></div>
            <div class="todo-stats">
                <span id="total-count">0 tasks</span>
                <span id="completed-count">0 completed</span>
            </div>
        </div>"#,
        vec![],
    );

    // Instantiate main container
    writer.write_instantiate(1, 1, 0);

    // Add 3 sample todos
    for i in 0..3 {
        writer.write_instantiate(10 + i, 0, 1);
        writer.write_patch_text(10 + i, 0, &format!("Sample task {}", i + 1));
    }

    // Update stats
    writer.write_patch_text(1, 1, "3 tasks");

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    writer.finish_and_sign(&signing_key).unwrap()
}

/// Generate HTIP binary for dashboard demo
pub fn generate_dashboard_htip() -> Vec<u8> {
    let mut writer = HtipWriter::new();

    // Template 0: Metric card
    writer.write_template(
        0,
        r#"<div class="metric-card">
            <div class="metric-label"></div>
            <div class="metric-value"></div>
            <div class="metric-change"></div>
        </div>"#,
        vec![],
    );

    // Template 1: Dashboard container
    writer.write_template(
        1,
        r#"<div class="dashboard">
            <h2>DX-WWW Dashboard</h2>
            <div class="metrics-grid" id="metrics"></div>
        </div>"#,
        vec![],
    );

    // Instantiate dashboard
    writer.write_instantiate(1, 1, 0);

    // Add metric cards
    let metrics = [
        ("Users", "12,543", "+8.2%"),
        ("Revenue", "$45,231", "+12.5%"),
        ("Sessions", "8,432", "-2.1%"),
        ("Conversion", "3.8%", "+0.4%"),
    ];

    for (i, (label, value, change)) in metrics.iter().enumerate() {
        let card_id = 10 + i as u32;
        writer.write_instantiate(card_id, 0, 1);
        writer.write_patch_text(card_id, 0, label);
        writer.write_patch_text(card_id, 1, value);
        writer.write_patch_text(card_id, 2, change);

        // Add color class based on change
        if change.starts_with('+') {
            writer.write_class_toggle(card_id, "positive", true);
        } else if change.starts_with('-') {
            writer.write_class_toggle(card_id, "negative", true);
        }
    }

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    writer.finish_and_sign(&signing_key).unwrap()
}
