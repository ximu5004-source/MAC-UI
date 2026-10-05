pub mod processing;

use slu_ipc::{ServiceIpc, messages::SvcAction};

use clap::{Arg, ArgAction, Command};

use crate::{
    SERVICE_DISPLAY_NAME,
    enviroment::{add_installation_dir_to_path, remove_installation_dir_from_path},
    error::Result,
    logger::SluServiceLogger,
    task_scheduler::TaskSchedulerHelper,
};

pub struct ServiceSubcommands;
impl ServiceSubcommands {
    pub const INSTALL: &str = "install";
    pub const UNINSTALL: &str = "uninstall";
    pub const STOP: &str = "stop";
}

pub fn get_cli() -> Command {
    let cli = Command::new(SERVICE_DISPLAY_NAME.to_string())
        .author("JONA")
        .about("MAC UI service command line interface.")
        .long_about("MAC UI service command line interface. Based on Seelen UI.")
        .before_help("")
        .after_help("MAC UI by JONA. Upstream attribution is included in NOTICE.md.")
        .subcommands([
            Command::new(ServiceSubcommands::INSTALL)
                .about("Installs or repairs the service (elevation required)."),
            Command::new(ServiceSubcommands::UNINSTALL)
                .about("Uninstalls the service (elevation required)."),
            Command::new(ServiceSubcommands::STOP).about("Stops the service."),
        ])
        .args([Arg::new("startup")
            .short('S')
            .long("startup")
            .action(ArgAction::SetTrue)
            .help("Indicates that the app was invoked from the start up action.")]);
    #[cfg(debug_assertions)]
    let cli = cli.arg(
        Arg::new("no-startup-task")
            .long("no-startup-task")
            .action(ArgAction::SetTrue)
            .help("Local QA only: leave the installed login task unchanged."),
    );
    cli
}

/// Handles the CLI and exits the process with 0 if it should
pub async fn handle_console_client() -> Result<()> {
    let matches = get_cli().get_matches();
    #[cfg(debug_assertions)]
    crate::SKIP_STARTUP_TASK.store(
        matches.get_flag("no-startup-task"),
        std::sync::atomic::Ordering::Release,
    );
    let subcommand = matches.subcommand();

    match subcommand {
        Some((ServiceSubcommands::INSTALL, _)) => {
            add_installation_dir_to_path()?;
            TaskSchedulerHelper::create_service_task()?;
        }
        Some((ServiceSubcommands::UNINSTALL, _)) => {
            SluServiceLogger::uninstall_old_logging()?;
            remove_installation_dir_from_path()?;
            TaskSchedulerHelper::remove_service_task()?;
        }
        Some((ServiceSubcommands::STOP, _)) => {
            ServiceIpc::send(SvcAction::Stop).await?;
        }
        _ => {}
    }

    if subcommand.is_some() {
        std::process::exit(0);
    }
    Ok(())
}

#[cfg(all(test, debug_assertions))]
mod qa_cli_tests {
    use super::get_cli;

    #[test]
    fn qa_mode_is_explicit_and_normal_startup_keeps_its_default() {
        let normal = get_cli().try_get_matches_from(["slu-service"]).unwrap();
        assert!(!normal.get_flag("no-startup-task"));
        let qa = get_cli()
            .try_get_matches_from(["slu-service", "--no-startup-task"])
            .unwrap();
        assert!(qa.get_flag("no-startup-task"));
        assert!(qa.subcommand().is_none());
    }
}
