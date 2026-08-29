#![forbid(unsafe_code)]

use linura_sdk::ProtocolVersion;
use std::env;

#[derive(Clone, Copy)]
struct CommandInfo {
    name: &'static str,
    summary: &'static str,
    offline: bool,
}

const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "version",
        summary: "Show Linura and protocol version",
        offline: true,
    },
    CommandInfo {
        name: "commands",
        summary: "List machine-readable CLI command metadata",
        offline: true,
    },
    CommandInfo {
        name: "help",
        summary: "Show CLI help",
        offline: true,
    },
];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "version" => {
            let version = ProtocolVersion::default();
            println!(
                "linuractl {} (protocol {})",
                version.product_version, version.major
            );
        }
        "commands" if args.get(1).map(String::as_str) == Some("--json") => print_commands_json(),
        "commands" => print_commands(),
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_commands() {
    for command in COMMANDS {
        println!("{:<12} {}", command.name, command.summary);
    }
}

fn print_commands_json() {
    print!("[");
    for (index, command) in COMMANDS.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!(
            "{{\"name\":\"{}\",\"summary\":\"{}\",\"offline\":{}}}",
            command.name, command.summary, command.offline
        );
    }
    println!("]");
}

fn print_help() {
    println!("linuractl - deterministic Linura control client");
    println!("\nUSAGE:\n  linuractl version\n  linuractl commands [--json]\n  linuractl help");
    println!(
        "\nPlanned command families: observe, intent, plan, apply, explain, graph, profile, update, recover."
    );
    println!("The CLI must remain usable without an AI/model provider.");
}
