use std::process::Command;

use crate::models::RestartPlan;

// Запускает помощника, который дождётся выхода процесса и поднимет новую версию; после вызова приложение должно завершиться.
pub fn restart(plan: &RestartPlan) -> Result<(), String> {
    let pid = std::process::id().to_string();
    let spawned = match plan {
        RestartPlan::Relaunch { app } => Command::new("/bin/sh")
            .args(["-c", r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; open "$2""#, "restart", &pid, app])
            .spawn(),
        RestartPlan::SilentInstaller { installer } => Command::new(installer).arg("/S").spawn(),
        RestartPlan::DebPackage { package } => Command::new("/bin/sh")
            .args([
                "-c",
                r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; pkexec dpkg -i "$2"; exec camunda-support"#,
                "restart",
                &pid,
                package,
            ])
            .spawn(),
    };
    spawned.map(|_| ()).map_err(|error| error.to_string())
}
