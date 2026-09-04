use std::error::Error;
use std::io;

use linura_executor_systemd::{QualificationUnitName, restart_effect};
use linura_provider_sdk::{ComponentDigest, ExecutionBinding};

fn main() -> Result<(), Box<dyn Error>> {
    let unit = std::env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: v05_binding <linura-v05-qualification-*.service>",
        )
    })?;
    let unit = QualificationUnitName::parse(unit)?;
    let effect = restart_effect(&unit)?;
    let binding = ExecutionBinding::new(
        "qualification:v0.5-vm",
        1,
        1,
        ComponentDigest::from_bytes([0x11; 32]),
        ComponentDigest::from_bytes([0x22; 32]),
        &effect,
    )?;

    println!("transaction_id={}", binding.transaction_id);
    println!("generation={}", binding.generation);
    println!("state_version={}", binding.state_version);
    println!(
        "authority_binding_digest={}",
        binding.authority_binding_digest.to_hex()
    );
    println!(
        "authority_use_digest={}",
        binding.authority_use_digest.to_hex()
    );
    println!("effect_digest={}", binding.effect_digest.to_hex());
    println!("dispatch_digest={}", binding.dispatch_digest.to_hex());
    Ok(())
}
