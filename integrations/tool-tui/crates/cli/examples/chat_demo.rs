use anyhow::Result;

fn main() -> Result<()> {
    let mut app = dx::ui::chat::ChatApp::new();
    app.run()
}
