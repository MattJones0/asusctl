use rog_dbus::RogDbusClient;
use rog_user::{
    ctrl_anime::{CtrlAnime, CtrlAnimeInner},
    user_config::*,
    DBUS_NAME,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use zbus::{fdo, Connection};

use std::sync::atomic::AtomicBool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("  user daemon v{}", rog_user::VERSION);
    println!("    rog-anime v{}", rog_anime::VERSION);
    println!("     rog-dbus v{}", rog_dbus::VERSION);
    println!("rog-supported v{}", rog_supported::VERSION);

    let (client, _) = RogDbusClient::new()?;
    let supported = client.proxies().supported().get_supported_functions()?;

    let mut config = UserConfig::new();
    config.load_config()?;

    let anime_config = UserAnimeConfig::load_config(config.active_anime)?;
    let anime = anime_config.create_anime()?;

    let anime_config = Arc::new(Mutex::new(anime_config));

    // Create server
    let connection = Connection::new_session()?;
    fdo::DBusProxy::new(&connection)?
        .request_name(DBUS_NAME, fdo::RequestNameFlags::ReplaceExisting.into())?;
    let mut server = zbus::ObjectServer::new(&connection);

    // Set up the anime data and run loop/thread
    if supported.anime_ctrl.0 {
        let early_return = Arc::new(AtomicBool::new(false));
        // Inner behind mutex required for thread safety
        let inner = Arc::new(Mutex::new(CtrlAnimeInner::new(
            anime,
            client,
            early_return.clone(),
        )?));
        // Need new client object for dbus control part
        let (client, _) = RogDbusClient::new()?;
        let anime_control = CtrlAnime::new(anime_config, inner.clone(), client, early_return)?;
        anime_control.add_to_server(&mut server);
        // Thread using inner
        let _anime_thread = thread::Builder::new()
            .name("Anime User".into())
            .spawn(move || loop {
                if let Ok(inner) = inner.try_lock() {
                    inner.run().ok();
                }
            })?;
    }

    if supported.keyboard_led.per_key_led_mode {}

    loop {
        if let Err(err) = server.try_handle_next() {
            println!("{}", err);
        }
    }
}
