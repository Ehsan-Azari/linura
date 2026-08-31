#![forbid(unsafe_code)]

use std::error::Error;

use linura_linux_observation::{NetworkManagerObserver, SystemdObserver};
use linura_observation_control::ObservationCoordinator;

fn main() {
    if let Err(error) = run() {
        eprintln!("linurad: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut coordinator = ObservationCoordinator::new();
    coordinator.register_observer(Box::new(SystemdObserver::connect()?))?;
    coordinator.register_observer(Box::new(NetworkManagerObserver::connect()?))?;
    linura_dbus::serve(coordinator)?;
    Ok(())
}
