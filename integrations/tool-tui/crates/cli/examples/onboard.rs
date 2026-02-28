//! Minimal onboarding example - builds only what's needed for onboarding

use dx::commands::onboard::run_onboard;

fn main() -> anyhow::Result<()> {
    run_onboard()
}
