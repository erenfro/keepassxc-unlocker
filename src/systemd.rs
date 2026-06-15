use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn add_service() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe
        .to_str()
        .context("Failed to get executable path")?;

    let unit_content = format!(
        r#"[Unit]
Description=KeePassXC Unlocker Service

[Service]
Type=simple
ExecStart={} watch

[Install]
WantedBy=default.target
"#,
        exe_path
    );

    let systemd_dir = get_user_systemd_dir()?;
    fs::create_dir_all(&systemd_dir)?;

    let service_file = systemd_dir.join("keepassxc-unlocker.service");
    fs::write(&service_file, unit_content)?;

    run_command("systemctl", &["--user", "daemon-reload"])?;
    run_command(
        "systemctl",
        &["--user", "enable", "--now", "keepassxc-unlocker.service"],
    )?;

    println!("Service installed and running");
    Ok(())
}

pub fn remove_service() -> Result<()> {
    run_command(
        "systemctl",
        &["--user", "disable", "--now", "keepassxc-unlocker.service"],
    )
    .ok();

    let systemd_dir = get_user_systemd_dir()?;
    let service_file = systemd_dir.join("keepassxc-unlocker.service");
    if service_file.exists() {
        fs::remove_file(service_file)?;
    }

    run_command("systemctl", &["--user", "daemon-reload"])?;
    println!("Service stopped and removed");
    Ok(())
}

pub fn status_service() -> Result<()> {
    let systemd_dir = get_user_systemd_dir()?;
    let service_file = systemd_dir.join("keepassxc-unlocker.service");

    if service_file.exists() {
        Command::new("systemctl")
            .args(&["--user", "status", "keepassxc-unlocker.service"])
            .status()?;
    } else {
        println!("Service is not installed");
    }
    Ok(())
}

fn get_user_systemd_dir() -> Result<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "keepassxc-unlocker") {
        let mut path = proj_dirs.config_dir().to_path_buf();
        path.pop(); // from ~/.config/keepassxc-unlocker to ~/.config
        path.push("systemd");
        path.push("user");
        Ok(path)
    } else {
        anyhow::bail!("Could not determine systemd directory")
    }
}

fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd).args(args).status().context(format!(
        "Failed to run {} {}",
        cmd,
        args.join(" ")
    ))?;

    if !status.success() {
        anyhow::bail!(
            "Command {} {} failed with exit code {:?}",
            cmd,
            args.join(" "),
            status.code()
        );
    }
    Ok(())
}
