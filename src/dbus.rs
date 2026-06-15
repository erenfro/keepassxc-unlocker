use anyhow::Result;
use futures_util::StreamExt;
use log::{error, info};
use zbus::{proxy, Connection};

#[proxy(
    interface = "org.keepassxc.KeePassXC.MainWindow",
    default_service = "org.keepassxc.KeePassXC.MainWindow",
    default_path = "/keepassxc"
)]
trait KeePassXC {
    #[zbus(name = "openDatabase")]
    fn open_database(&self, public_key: &str, password: &str) -> zbus::Result<()>;
}

pub mod gnome {
    use zbus::proxy;
    #[proxy(
        interface = "org.gnome.ScreenSaver",
        default_service = "org.gnome.ScreenSaver",
        default_path = "/org/gnome/ScreenSaver"
    )]
    pub trait GnomeScreenSaver {
        #[zbus(signal)]
        fn active_changed(&self, is_active: bool) -> zbus::Result<()>;
    }
}

pub mod freedesktop {
    use zbus::proxy;
    #[proxy(
        interface = "org.freedesktop.ScreenSaver",
        default_service = "org.freedesktop.ScreenSaver",
        default_path = "/org/freedesktop/ScreenSaver"
    )]
    pub trait FreedesktopScreenSaver {
        #[zbus(signal)]
        fn active_changed(&self, is_active: bool) -> zbus::Result<()>;
    }
}

pub async fn unlock_database(database: &str, password: &str, silent: bool) -> Result<()> {
    let connection = Connection::session().await?;
    let proxy = KeePassXCProxy::new(&connection).await?;
    proxy.open_database(database, password).await?;
    if !silent {
        info!("Successfully sent request to unlock database: {}", database);
    }
    Ok(())
}

pub async fn monitor_screensaver<F>(mut on_state_changed: F) -> Result<()>
where
    F: FnMut(bool) + Send + 'static,
{
    let connection = Connection::session().await?;

    let gnome_proxy = gnome::GnomeScreenSaverProxy::new(&connection).await.ok();
    let fd_proxy = freedesktop::FreedesktopScreenSaverProxy::new(&connection)
        .await
        .ok();

    if gnome_proxy.is_none() && fd_proxy.is_none() {
        error!("No suitable screensaver interfaces were found.");
        return Ok(());
    }

    let mut gnome_stream = if let Some(p) = gnome_proxy {
        info!("Subscribed to org.gnome.ScreenSaver at /org/gnome/ScreenSaver");
        Some(p.receive_active_changed().await?)
    } else {
        None
    };

    let mut fd_stream = if let Some(p) = fd_proxy {
        info!("Subscribed to org.freedesktop.ScreenSaver at /org/freedesktop/ScreenSaver");
        Some(p.receive_active_changed().await?)
    } else {
        None
    };

    info!("Listening for screensaver events...");

    loop {
        tokio::select! {
            Some(signal) = async {
                if let Some(ref mut s) = gnome_stream {
                    s.next().await
                } else {
                    futures_util::future::pending().await
                }
            } => {
                if let Ok(args) = signal.args() {
                    let is_active = *args.is_active();
                    handle_active_changed(is_active, &mut on_state_changed);
                }
            }
            Some(signal) = async {
                if let Some(ref mut s) = fd_stream {
                    s.next().await
                } else {
                    futures_util::future::pending().await
                }
            } => {
                if let Ok(args) = signal.args() {
                    let is_active = *args.is_active();
                    handle_active_changed(is_active, &mut on_state_changed);
                }
            }
        }
    }
}

fn handle_active_changed<F>(is_active: bool, on_state_changed: &mut F)
where
    F: FnMut(bool),
{
    if is_active {
        info!("Session is locked.");
    } else {
        info!("Session is unlocked.");
    }
    on_state_changed(is_active);
}
