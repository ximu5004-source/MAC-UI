use windows::Win32::{
    Foundation::{VARIANT_FALSE, VARIANT_TRUE},
    System::TaskScheduler::{
        IExecAction2, ITaskFolder, ITaskService, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
        TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_TRIGGER_LOGON, TaskScheduler,
    },
    UI::Shell::FOLDERID_LocalAppData,
};
use windows_core::{BSTR, Interface};

mod ownership;

use crate::{
    enviroment::was_installed_using_msix,
    error::Result,
    windows_api::{WindowsApi, com::Com},
};

pub struct TaskSchedulerHelper {}

static GROUP_FOLDER: &str = "\\MAC UI";
static SERVICE_TASK_NAME: &str = "MAC UI Service";

impl TaskSchedulerHelper {
    /// Launching the helper must not require rewriting an already-correct
    /// elevated task. Standard-user launches may read/run it but cannot update it.
    pub fn ensure_service_task() -> Result<()> {
        let service_path = if was_installed_using_msix() {
            WindowsApi::known_folder(FOLDERID_LocalAppData)?
                .join("Microsoft\\WindowsApps\\macui-service.exe")
        } else {
            std::env::current_exe()?
        };
        let reusable = Com::run_with_context(|| unsafe {
            let service = Self::get_task_service()?;
            let task = service
                .GetFolder(&GROUP_FOLDER.into())?
                .GetTask(&SERVICE_TASK_NAME.into())?;
            let definition = task.Definition()?;
            let actions = definition.Actions()?;
            let action: IExecAction2 = actions.get_Item(1)?.cast()?;
            let mut path = BSTR::new();
            let mut args = BSTR::new();
            action.Path(&mut path)?;
            action.Arguments(&mut args)?;
            let mut level = Default::default();
            definition.Principal()?.RunLevel(&mut level)?;
            let mut demand = VARIANT_FALSE;
            definition.Settings()?.AllowDemandStart(&mut demand)?;
            let mut count = 0;
            actions.Count(&mut count)?;
            Ok(count == 1
                && reusable_task(
                    &path.to_string(),
                    &service_path.to_string_lossy(),
                    &args.to_string(),
                    level == TASK_RUNLEVEL_HIGHEST,
                    task.Enabled()? != VARIANT_FALSE,
                    demand != VARIANT_FALSE,
                ))
        })
        .unwrap_or(false);
        if reusable {
            log::debug!("Existing service task matches this installation; keeping it unchanged");
            return Ok(());
        }
        Self::create_service_task()
    }

    unsafe fn get_task_service() -> Result<ITaskService> {
        unsafe {
            let task_service: ITaskService = Com::create_instance(&TaskScheduler)?;
            task_service.Connect(
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )?;
            Ok(task_service)
        }
    }

    unsafe fn register_task(folder: &ITaskFolder, task_name: &str, task_xml: &BSTR) -> Result<()> {
        unsafe {
            folder.RegisterTask(
                &task_name.into(),
                task_xml,
                TASK_CREATE_OR_UPDATE.0,
                &Default::default(),
                &Default::default(),
                TASK_LOGON_INTERACTIVE_TOKEN,
                &Default::default(),
            )?;
            log::debug!("Registered task {}", task_name);
            Ok(())
        }
    }

    /// this task handles the startup of the service and the app on login
    pub fn create_service_task() -> Result<()> {
        log::debug!("Creating service task...");
        let service_path = if was_installed_using_msix() {
            WindowsApi::known_folder(FOLDERID_LocalAppData)?
                .join("Microsoft\\WindowsApps\\macui-service.exe")
        } else {
            std::env::current_exe()?
        };
        Com::run_with_context(|| unsafe {
            let task_service = Self::get_task_service()?;
            let existing = task_service
                .GetFolder(&GROUP_FOLDER.into())
                .and_then(|folder| folder.GetTask(&SERVICE_TASK_NAME.into()))
                .ok();
            // Adopt only an earlier task owned by this installation directory.
            // A separately installed upstream application is never changed.
            let legacy_folder = task_service.GetFolder(&"\\Seelen".into()).ok();
            let legacy = legacy_folder
                .as_ref()
                .and_then(|folder| folder.GetTask(&"Seelen UI Service".into()).ok())
                .filter(|task| {
                    let path = (|| -> Result<String> {
                        let action: IExecAction2 =
                            task.Definition()?.Actions()?.get_Item(1)?.cast()?;
                        let mut value = BSTR::new();
                        action.Path(&mut value)?;
                        Ok(value.to_string())
                    })();
                    path.is_ok_and(|path| {
                        ownership::is_owned_service_action(
                            std::path::Path::new(&path),
                            &service_path,
                            !was_installed_using_msix(),
                        )
                    })
                });
            let migrating_legacy = legacy.is_some();
            let old_task = existing.or(legacy);
            let root_folder = task_service.GetFolder(&"\\".into())?;
            let folder = task_service
                .GetFolder(&GROUP_FOLDER.into())
                .or_else(|_| root_folder.CreateFolder(&GROUP_FOLDER.into(), &Default::default()))?;

            let task = task_service.NewTask(0)?;
            task.Principal()?.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

            let settings = task.Settings()?;
            settings.SetPriority(2)?;
            settings.SetHidden(VARIANT_TRUE)?;
            settings.SetAllowDemandStart(VARIANT_TRUE)?;
            settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
            settings.SetStopIfGoingOnBatteries(VARIANT_FALSE)?;

            let triggers = task.Triggers()?;
            if let Some(old) = old_task {
                let old_triggers = old.Definition()?.Triggers()?;
                task.SetTriggers(&old_triggers)?;
            } else {
                triggers.Create(TASK_TRIGGER_LOGON)?;
            }

            let actions = task.Actions()?;
            let exec_action: IExecAction2 = actions.Create(TASK_ACTION_EXEC)?.cast()?;
            exec_action.SetPath(&service_path.to_string_lossy().to_string().into())?;
            exec_action.SetArguments(&"--startup".into())?;

            let mut task_xml = BSTR::new();
            task.XmlText(&mut task_xml)?;
            Self::register_task(&folder, SERVICE_TASK_NAME, &task_xml)?;
            if migrating_legacy {
                if let Some(old_folder) = legacy_folder {
                    old_folder.DeleteTask(&"Seelen UI Service".into(), 0)?;
                }
            }
            Ok(())
        })
    }

    pub fn set_run_on_logon(enabled: bool) -> Result<()> {
        Com::run_with_context(|| unsafe {
            let task_service = Self::get_task_service()?;
            let macui_folder = task_service.GetFolder(&GROUP_FOLDER.into())?;
            let task = macui_folder.GetTask(&SERVICE_TASK_NAME.into())?;
            let task = task.Definition()?;
            let triggers = task.Triggers()?;
            triggers.Clear()?;
            if enabled {
                triggers.Create(TASK_TRIGGER_LOGON)?;
            }
            let mut task_xml = BSTR::new();
            task.XmlText(&mut task_xml)?;
            Self::register_task(&macui_folder, SERVICE_TASK_NAME, &task_xml)?;
            Ok(())
        })
    }

    pub fn remove_service_task() -> Result<()> {
        Com::run_with_context(|| unsafe {
            let task_service: ITaskService = Com::create_instance(&TaskScheduler)?;
            task_service.Connect(
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default(),
            )?;
            if let Ok(macui_folder) = task_service.GetFolder(&GROUP_FOLDER.into()) {
                let _ = macui_folder.DeleteTask(&SERVICE_TASK_NAME.into(), 0);
            }
            Ok(())
        })
    }
}

fn reusable_task(
    path: &str,
    expected: &str,
    arguments: &str,
    highest: bool,
    enabled: bool,
    demand: bool,
) -> bool {
    path.eq_ignore_ascii_case(expected)
        && arguments.trim() == "--startup"
        && highest
        && enabled
        && demand
}

#[cfg(test)]
mod startup_task_tests {
    use super::reusable_task;
    #[test]
    fn correct_owned_task_is_preserved_without_elevated_re_registration() {
        let expected = "C:\\Apps\\MAC UI\\macui-service.exe";
        assert!(reusable_task(
            "c:\\apps\\mac ui\\MACUI-SERVICE.EXE",
            expected,
            "--startup",
            true,
            true,
            true
        ));
        assert!(!reusable_task(
            "D:\\Other\\macui-service.exe",
            expected,
            "--startup",
            true,
            true,
            true
        ));
        assert!(!reusable_task(
            expected,
            expected,
            "--startup --other",
            true,
            true,
            true
        ));
        assert!(!reusable_task(
            expected,
            expected,
            "--startup",
            false,
            true,
            true
        ));
        assert!(!reusable_task(
            expected,
            expected,
            "--startup",
            true,
            false,
            true
        ));
        assert!(!reusable_task(
            expected,
            expected,
            "--startup",
            true,
            true,
            false
        ));
    }
}
