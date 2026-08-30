#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

use linura_sdk::{LocalControlClient, ProtocolVersion};

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
        name: "whoami",
        summary: "Show the authenticated local D-Bus caller identity",
        offline: false,
    },
    CommandInfo {
        name: "capabilities",
        summary: "List observation providers, health, and capabilities",
        offline: false,
    },
    CommandInfo {
        name: "observe",
        summary: "Read authoritative current state for one resource",
        offline: false,
    },
    CommandInfo {
        name: "graph",
        summary: "Show the current observed causal system graph",
        offline: false,
    },
    CommandInfo {
        name: "explain",
        summary: "Explain the authoritative evidence for one resource",
        offline: false,
    },
    CommandInfo {
        name: "help",
        summary: "Show CLI help",
        offline: true,
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("linuractl: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("version") => {
            require_arity(&args, 1, "version")?;
            let version = ProtocolVersion::default();
            println!(
                "linuractl {} (protocol {})",
                version.product_version, version.major
            );
        }
        Some("commands") if args.get(1).map(String::as_str) == Some("--json") => {
            require_arity(&args, 2, "commands [--json]")?;
            println!("{}", commands_json());
        }
        Some("commands") => {
            require_arity(&args, 1, "commands [--json]")?;
            print_commands();
        }
        Some("whoami") => {
            require_arity(&args, 1, "whoami")?;
            let (actor_id, kind, interactive, uid, pid, sender) =
                LocalControlClient::connect()?.who_am_i()?;
            field("actor_id", &actor_id);
            field("actor_kind", &kind);
            field("interactive", if interactive { "true" } else { "false" });
            field("uid", &uid.to_string());
            field("pid", &pid.to_string());
            field("dbus_sender", &sender);
        }
        Some("capabilities") => {
            require_arity(&args, 1, "capabilities")?;
            let (providers, capabilities) = LocalControlClient::connect()?.capabilities()?;
            for (index, (provider, availability, reason)) in providers.iter().enumerate() {
                if index > 0 {
                    println!();
                }
                field("provider", provider);
                field("availability", availability);
                if !reason.is_empty() {
                    field("reason", reason);
                }
            }
            for (id, provider, support, reason) in capabilities {
                println!();
                field("capability", &id);
                field("provider", &provider);
                field("support", &support);
                if !reason.is_empty() {
                    field("reason", &reason);
                }
            }
        }
        Some("observe") => {
            let (provider, capability, resource) = parse_observe_args(&args)?;
            let observation =
                LocalControlClient::connect()?.observe(provider, resource, capability)?;
            let (
                provider,
                resource,
                capability,
                authority,
                freshness,
                observed_at,
                valid_for,
                sequence,
                attributes,
            ) = observation;
            field("provider", &provider);
            field("resource", &resource);
            field("capability", &capability);
            field("authority", &authority);
            field("freshness", &freshness);
            field("observed_at_unix_ms", &observed_at.to_string());
            field("valid_for_ms", &valid_for.to_string());
            field("sequence", &sequence.to_string());
            for (key, value) in attributes {
                field(&format!("attribute.{key}"), &value);
            }
        }
        Some("graph") => {
            require_arity(&args, 1, "graph")?;
            let (nodes, edges) = LocalControlClient::connect()?.graph()?;
            for (index, (node, attributes)) in nodes.into_iter().enumerate() {
                if index > 0 {
                    println!();
                }
                field("node", &node);
                for (key, value) in attributes {
                    field(&format!("attribute.{key}"), &value);
                }
            }
            for (from, to, kind, reason) in edges {
                println!();
                field("edge_from", &from);
                field("edge_to", &to);
                field("edge_kind", &kind);
                field("reason", &reason);
            }
        }
        Some("explain") => {
            require_arity(&args, 2, "explain <resource>")?;
            let (resource, provider, capability, freshness, evidence_id, authority) =
                LocalControlClient::connect()?.explain(&args[1])?;
            field("resource", &resource);
            field("provider", &provider);
            field("capability", &capability);
            field("freshness", &freshness);
            field("evidence_id", &evidence_id);
            field("authority", &authority);
        }
        Some("help") | Some("--help") | Some("-h") | None => print_help(),
        Some(other) => {
            return Err(Box::new(CliError(format!(
                "unknown command {other:?}; run `linuractl help`"
            ))));
        }
    }
    Ok(())
}

fn parse_observe_args(args: &[String]) -> Result<(&str, &str, &str), Box<dyn Error>> {
    match args {
        [_, resource] => {
            let (provider, capability) = infer_route(resource)?;
            Ok((provider, capability, resource))
        }
        [_, provider, capability, resource] => Ok((provider, capability, resource)),
        _ => Err(Box::new(CliError(
            "usage: linuractl observe <resource> OR linuractl observe <provider> <capability> <resource>"
                .into(),
        ))),
    }
}

fn infer_route(resource: &str) -> Result<(&'static str, &'static str), Box<dyn Error>> {
    if resource.starts_with("systemd:unit:") {
        return Ok(("systemd", "systemd.unit.observe"));
    }
    if resource == "networkmanager:manager" {
        return Ok(("networkmanager", "networkmanager.manager.observe"));
    }
    if resource.starts_with("networkmanager:device:") {
        return Ok(("networkmanager", "networkmanager.device.observe"));
    }
    Err(Box::new(CliError(format!(
        "cannot infer an observation provider for {resource:?}; use `linuractl observe <provider> <capability> <resource>`"
    ))))
}

fn require_arity(args: &[String], expected: usize, usage: &str) -> Result<(), Box<dyn Error>> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(Box::new(CliError(format!("usage: linuractl {usage}"))))
    }
}

fn print_commands() {
    for command in COMMANDS {
        println!("{:<14} {}", command.name, command.summary);
    }
}

fn commands_json() -> String {
    let mut output = String::from("[");
    for (index, command) in COMMANDS.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"name\":");
        output.push_str(&json_string(command.name));
        output.push_str(",\"summary\":");
        output.push_str(&json_string(command.summary));
        output.push_str(",\"offline\":");
        output.push_str(if command.offline { "true" } else { "false" });
        output.push('}');
    }
    output.push(']');
    output
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                use std::fmt::Write as _;
                write!(&mut output, "\\u{:04x}", u32::from(character))
                    .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn field(key: &str, value: &str) {
    println!("{}={}", escaped(key), escaped(value));
}

fn escaped(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output
}

fn print_help() {
    println!("linuractl {}", env!("CARGO_PKG_VERSION"));
    println!("Read-only Linura v0.0.1 observation client.");
    println!();
    println!("Commands:");
    println!("  version");
    println!("  commands [--json]");
    println!("  whoami");
    println!("  capabilities");
    println!("  observe <resource>");
    println!("  observe <provider> <capability> <resource>");
    println!("  graph");
    println!("  explain <resource>");
    println!("  help");
}

#[derive(Debug)]
struct CliError(String);

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CliError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn built_in_resource_routes_are_deterministic() {
        assert_eq!(
            infer_route("systemd:unit:sshd.service")
                .unwrap_or_else(|error| unreachable!("{error}")),
            ("systemd", "systemd.unit.observe")
        );
        assert_eq!(
            infer_route("networkmanager:manager").unwrap_or_else(|error| unreachable!("{error}")),
            ("networkmanager", "networkmanager.manager.observe")
        );
    }

    #[test]
    fn output_escaping_is_line_safe() {
        assert_eq!(escaped("a\nb\\c\t"), "a\\nb\\\\c\\t");
    }

    #[test]
    fn command_catalog_is_unique_and_keeps_json_introspection() {
        let names: BTreeSet<_> = COMMANDS.iter().map(|command| command.name).collect();
        assert_eq!(names.len(), COMMANDS.len());
        assert!(names.contains("commands"));
        assert!(names.contains("observe"));

        let json = commands_json();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("\"name\":\"commands\""));
        assert!(json.contains("\"name\":\"observe\""));
        assert!(json.contains("\"offline\":true"));
        assert!(json.contains("\"offline\":false"));
    }

    #[test]
    fn json_strings_escape_control_characters() {
        assert_eq!(json_string("a\n\"b\\c\t"), "\"a\\n\\\"b\\\\c\\t\"");
    }
}