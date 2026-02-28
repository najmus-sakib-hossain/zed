//! Project lifecycle commands (init, dev, build, run, test, deploy)

mod animate;
mod build;
mod clean;
mod completions;
mod deploy;
mod dev;
mod info;
mod init;
mod run;
mod self_cmd;
mod shell;
mod test;

pub use animate::run_animate;
pub use build::run_build;
pub use clean::run_clean;
pub use completions::run_completions;
pub use deploy::run_deploy;
pub use dev::run_dev;
pub use info::run_info;
pub use init::run_init;
pub use run::run_run;
pub use shell::run_shell;
pub use test::run_test;
