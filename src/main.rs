mod config;
mod dbus;
mod keyring;
mod monitor;
mod systemd;

use anyhow::Result;
use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use log::{error, info};
use rpassword::read_password;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

#[derive(Parser)]
#[command(name = "keepassxc-unlocker")]
#[command(about = "Automatically unlocks KeePassXC databases", long_about = None)]
#[command(version)] // Enables --version
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Show version information
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an entry to the keyring
    Add {
        /// Path to the KeePassXC database file
        database: String,
    },
    /// Remove an entry from the keyring
    Remove {
        /// Path to the KeePassXC database file
        database: String,
    },
    /// List configured databases
    List,
    /// Generate shell completion scripts
    Completion {
        /// The shell to generate completions for
        shell: Shell,
    },
    /// Manage the systemd user service
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Unlock all KeePassXC databases from keyring
    Unlock,
    /// Monitor screensaver lock/unlock events
    Watch,
    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum ServiceAction {
    /// Add systemd user service to automate unlock
    Add,
    /// Remove systemd user service and automation
    Remove,
    /// Status of service
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // If no command is provided, clap would usually show help,
            // but we can also handle the --version case here if needed.
            // Actually, ArgAction::Version will exit automatically.
            return Ok(());
        }
    };

    match command {
        Commands::Add { database } => add_database(&database).await?,
        Commands::Remove { database } => remove_database(&database).await?,
        Commands::List => list_databases()?,
        Commands::Completion { shell } => {
            print_completions(shell, &mut Cli::command());
        }
        Commands::Service { action } => match action {
            ServiceAction::Add => systemd::add_service()?,
            ServiceAction::Remove => systemd::remove_service()?,
            ServiceAction::Status => systemd::status_service()?,
        },
        Commands::Unlock => unlock_all(false).await?,
        Commands::Watch => watch().await?,
        Commands::Version => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn list_databases() -> Result<()> {
    let config = config::Config::load()?;
    let databases = config.get_databases();

    if databases.is_empty() {
        println!("No databases configured.");
        return Ok(());
    }

    println!("{:<60} | {:<10}", "Database Path", "Status");
    println!("{:-<60}-+-{:-<10}", "", "");
    for (database, status) in databases {
        println!("{:<60} | {:<10}", database, status);
    }
    Ok(())
}

async fn add_database(database: &str) -> Result<()> {
    let mut config = config::Config::load()?;
    let kr = keyring::Keyring::new(config.get_service_name());

    print!("Password: ");
    io::stdout().flush()?;
    let password = read_password()?;

    kr.set_password(database, &password)?;
    println!("Added entry to keyring for database: {}", database);

    config.add_database(database);
    config.save()?;
    Ok(())
}

async fn remove_database(database: &str) -> Result<()> {
    let mut config = config::Config::load()?;
    let kr = keyring::Keyring::new(config.get_service_name());

    match kr.delete_password(database) {
        Ok(_) => println!("Removed entry from keyring"),
        Err(_) => println!("No entry found in keyring for database"),
    }

    config.remove_database(database);
    config.save()?;
    println!("Removed database from configuration");
    Ok(())
}

async fn unlock_all(silent: bool) -> Result<()> {
    let config = config::Config::load()?;
    let kr = keyring::Keyring::new(config.get_service_name());
    let databases = config.get_databases();

    for (database, status) in databases {
        if status.to_lowercase() != "enabled" {
            continue;
        }

        match kr.get_password(&database) {
            Ok(password) => {
                if !silent {
                    info!("Attempting to unlock database: {}", database);
                }
                if let Err(e) = dbus::unlock_database(&database, &password, silent).await {
                    error!("Failed to unlock database {}: {:?}", database, e);
                }
            }
            Err(e) => {
                error!("No password found for database {}: {:?}", database, e);
            }
        }
    }
    Ok(())
}

async fn watch() -> Result<()> {
    let config = config::Config::load()?;
    let process_name = config.get_process_name();

    // Track session lock state to avoid spamming while locked
    let session_locked = Arc::new(Mutex::new(false));

    let lock_state_for_proc = session_locked.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor::monitor_process(process_name, move || {
            let lock_state = lock_state_for_proc.clone();
            tokio::spawn(async move {
                // If session is locked, don't try to unlock database yet
                if !*lock_state.lock().unwrap() {
                    if let Err(e) = unlock_all(false).await {
                        error!("Failed to unlock databases on process find: {}", e);
                    }
                }
            });
        })
        .await;
    });

    let lock_state_for_dbus = session_locked.clone();
    let dbus_handle = tokio::spawn(async move {
        dbus::monitor_screensaver(move |is_active| {
            let lock_state = lock_state_for_dbus.clone();
            *lock_state.lock().unwrap() = is_active;

            if !is_active {
                tokio::spawn(async move {
                    if let Err(e) = unlock_all(false).await {
                        error!("Failed to unlock databases on screensaver unlock: {}", e);
                    }
                });
            }
        })
        .await
    });

    // Periodic check task (based on config)
    let autounlock_interval = config.get_autounlock_interval();
    let lock_state_for_periodic = session_locked.clone();
    let periodic_handle = tokio::spawn(async move {
        if autounlock_interval > 0 {
            info!(
                "Periodic auto-unlock enabled every {} seconds.",
                autounlock_interval
            );
            loop {
                sleep(Duration::from_secs(autounlock_interval)).await;
                if !*lock_state_for_periodic.lock().unwrap() {
                    // Periodic check is silent
                    let _ = unlock_all(true).await;
                }
            }
        } else {
            // If disabled, just sleep indefinitely to not trigger select
            futures_util::future::pending::<()>().await;
        }
    });

    tokio::select! {
        _ = monitor_handle => {},
        _ = dbus_handle => {},
        _ = periodic_handle => {},
    }

    Ok(())
}
