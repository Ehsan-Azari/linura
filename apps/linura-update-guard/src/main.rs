#![forbid(unsafe_code)]

use std::env;
use linura_update::{direct_upgrade_decision, NativeUpgradeDecision};

fn truthy(name: &str) -> bool {
    matches!(env::var(name).as_deref(), Ok("1" | "true" | "yes" | "on"))
}

fn main() {
    let coordinator = truthy("LINURA_UPDATE_CONTEXT");
    let break_glass = truthy("LINURA_ALLOW_DIRECT_PACMAN");
    match direct_upgrade_decision(coordinator, break_glass) {
        NativeUpgradeDecision::AllowLinuraCoordinator => {}
        NativeUpgradeDecision::AllowBreakGlass => {
            eprintln!("warning: Linura coordinated update safeguards are bypassed by explicit break-glass override");
        }
        NativeUpgradeDecision::DenyDirectUpgrade => {
            eprintln!("Linura blocks direct package upgrades so snapshots, migrations, reconciliation, and verification cannot be skipped.");
            eprintln!("Use the Linura update coordinator, or set LINURA_ALLOW_DIRECT_PACMAN=1 only for deliberate break-glass recovery.");
            std::process::exit(78);
        }
    }
}
