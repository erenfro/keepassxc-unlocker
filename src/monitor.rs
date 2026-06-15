use log::{info, warn};
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use tokio::time::sleep;

pub async fn monitor_process<F>(process_name: String, mut on_process_found: F)
where
    F: FnMut() + Send + 'static,
{
    let mut sys =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    let mut last_pid = None;

    info!("Monitoring process: {}...", process_name);

    loop {
        sys.refresh_processes(ProcessesToUpdate::All);

        let current_process = sys
            .processes()
            .values()
            .find(|p| p.name().to_string_lossy() == process_name);

        if let Some(process) = current_process {
            let pid = process.pid();
            if Some(pid) != last_pid {
                info!("KeePassXC process found with PID: {}", pid);
                last_pid = Some(pid);
                on_process_found();
            }
        } else {
            if last_pid.is_some() {
                warn!("KeePassXC process not found");
                last_pid = None;
            }
        }

        sleep(Duration::from_secs(5)).await;
    }
}
