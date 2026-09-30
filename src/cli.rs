use std::sync::Mutex;

use clap::{Arg, ArgAction, ArgMatches, Command, error::ErrorKind};

use crate::origout;

#[derive(Default)]
pub struct CliService(Mutex<CliState>);

#[derive(Default)]
struct CliState {
    commands: Vec<Command>,
    arguments: Vec<Arg>,
    matches: Vec<ArgMatches>,
}

fn index(handle: i64, len: usize) -> usize {
    let index = usize::try_from(handle - 1).expect("invalid CLI handle");
    assert!(index < len, "invalid CLI handle");
    index
}

impl origout::OrigoutCliCapability for CliService {
    fn new_command(&self, name: String) -> i64 {
        let mut state = self.0.lock().unwrap();
        state.commands.push(Command::new(name));
        state.commands.len() as i64
    }

    fn set_about(&self, command: i64, about: String) {
        let mut state = self.0.lock().unwrap();
        let slot = index(command, state.commands.len());
        state.commands[slot] = state.commands[slot].clone().about(about);
    }

    fn set_version(&self, command: i64, version: String) {
        let mut state = self.0.lock().unwrap();
        let slot = index(command, state.commands.len());
        state.commands[slot] = state.commands[slot].clone().version(version);
    }

    fn require_subcommand(&self, command: i64) {
        let mut state = self.0.lock().unwrap();
        let slot = index(command, state.commands.len());
        state.commands[slot] = state.commands[slot].clone().subcommand_required(true);
    }

    fn new_argument(&self, name: String, kind: String, value_type: String) -> i64 {
        let mut arg = Arg::new(name);
        arg = match (kind.as_str(), value_type.as_str()) {
            ("flag", "bool") => arg.action(ArgAction::SetTrue),
            ("option" | "positional", "str") => arg.value_parser(clap::value_parser!(String)),
            ("option" | "positional", "int") => arg.value_parser(clap::value_parser!(i64)),
            ("option" | "positional", "float") => arg.value_parser(clap::value_parser!(f64)),
            ("option" | "positional", "bool") => arg.value_parser(clap::value_parser!(bool)),
            _ => panic!("unsupported CLI argument kind or value type"),
        };
        let mut state = self.0.lock().unwrap();
        state.arguments.push(arg);
        state.arguments.len() as i64
    }

    fn set_short(&self, argument: i64, short_name: String) {
        let mut state = self.0.lock().unwrap();
        let slot = index(argument, state.arguments.len());
        let short = short_name
            .chars()
            .next()
            .expect("short name must be one character");
        assert_eq!(
            short_name.chars().count(),
            1,
            "short name must be one character"
        );
        state.arguments[slot] = state.arguments[slot].clone().short(short);
    }

    fn set_long(&self, argument: i64, long_name: String) {
        let mut state = self.0.lock().unwrap();
        let slot = index(argument, state.arguments.len());
        state.arguments[slot] = state.arguments[slot].clone().long(long_name);
    }

    fn set_help(&self, argument: i64, help: String) {
        let mut state = self.0.lock().unwrap();
        let slot = index(argument, state.arguments.len());
        state.arguments[slot] = state.arguments[slot].clone().help(help);
    }

    fn set_required(&self, argument: i64, required: bool) {
        let mut state = self.0.lock().unwrap();
        let slot = index(argument, state.arguments.len());
        state.arguments[slot] = state.arguments[slot].clone().required(required);
    }

    fn add_argument(&self, command: i64, argument: i64) {
        let mut state = self.0.lock().unwrap();
        let command_slot = index(command, state.commands.len());
        let argument_slot = index(argument, state.arguments.len());
        state.commands[command_slot] = state.commands[command_slot]
            .clone()
            .arg(state.arguments[argument_slot].clone());
    }

    fn add_subcommand(&self, command: i64, subcommand: i64) {
        let mut state = self.0.lock().unwrap();
        let command_slot = index(command, state.commands.len());
        let subcommand_slot = index(subcommand, state.commands.len());
        state.commands[command_slot] = state.commands[command_slot]
            .clone()
            .subcommand(state.commands[subcommand_slot].clone());
    }

    fn parse(&self, command: i64, argv: Vec<String>) -> origout::CliParseResult {
        let mut state = self.0.lock().unwrap();
        let slot = index(command, state.commands.len());
        match state.commands[slot].clone().try_get_matches_from(argv) {
            Ok(matches) => {
                state.matches.push(matches);
                origout::CliParseResult {
                    exit_code: 0,
                    matches: state.matches.len() as i64,
                    output: String::new(),
                }
            }
            Err(error) => origout::CliParseResult {
                exit_code: if matches!(
                    error.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                ) {
                    0
                } else {
                    2
                },
                matches: 0,
                output: error.to_string(),
            },
        }
    }

    fn subcommand_name(&self, matches: i64) -> String {
        let state = self.0.lock().unwrap();
        state.matches[index(matches, state.matches.len())]
            .subcommand_name()
            .unwrap_or("")
            .to_owned()
    }

    fn subcommand_matches(&self, matches: i64) -> i64 {
        let mut state = self.0.lock().unwrap();
        let slot = index(matches, state.matches.len());
        if let Some((_, submatches)) = state.matches[slot].subcommand() {
            let submatches = submatches.clone();
            state.matches.push(submatches);
            state.matches.len() as i64
        } else {
            0
        }
    }

    fn get_string(&self, matches: i64, name: String) -> Option<String> {
        let state = self.0.lock().unwrap();
        state.matches[index(matches, state.matches.len())]
            .try_get_one::<String>(&name)
            .ok()
            .flatten()
            .cloned()
    }

    fn get_int(&self, matches: i64, name: String) -> Option<i64> {
        let state = self.0.lock().unwrap();
        state.matches[index(matches, state.matches.len())]
            .try_get_one::<i64>(&name)
            .ok()
            .flatten()
            .copied()
    }

    fn get_float(&self, matches: i64, name: String) -> Option<f64> {
        let state = self.0.lock().unwrap();
        state.matches[index(matches, state.matches.len())]
            .try_get_one::<f64>(&name)
            .ok()
            .flatten()
            .copied()
    }

    fn get_bool(&self, matches: i64, name: String) -> Option<bool> {
        let state = self.0.lock().unwrap();
        state.matches[index(matches, state.matches.len())]
            .try_get_one::<bool>(&name)
            .ok()
            .flatten()
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use origout::OrigoutCliCapability;

    #[test]
    fn subcommand_parses_typed_arguments() {
        let cli = CliService::default();
        let root = cli.new_command("origout".into());
        let inspect = cli.new_command("inspect".into());

        let name = cli.new_argument("name".into(), "positional".into(), "str".into());
        cli.set_required(name, true);
        cli.add_argument(inspect, name);

        let count = cli.new_argument("count".into(), "option".into(), "int".into());
        cli.set_short(count, "c".into());
        cli.set_long(count, "count".into());
        cli.add_argument(inspect, count);

        let ratio = cli.new_argument("ratio".into(), "option".into(), "float".into());
        cli.set_long(ratio, "ratio".into());
        cli.add_argument(inspect, ratio);

        let enabled = cli.new_argument("enabled".into(), "option".into(), "bool".into());
        cli.set_long(enabled, "enabled".into());
        cli.add_argument(inspect, enabled);

        let verbose = cli.new_argument("verbose".into(), "flag".into(), "bool".into());
        cli.set_long(verbose, "verbose".into());
        cli.add_argument(inspect, verbose);
        cli.add_subcommand(root, inspect);

        let parsed = cli.parse(
            root,
            [
                "origout",
                "inspect",
                "alice",
                "-c",
                "7",
                "--ratio",
                "0.5",
                "--enabled",
                "false",
                "--verbose",
            ]
            .map(str::to_owned)
            .to_vec(),
        );
        assert_eq!(parsed.exit_code, 0);
        assert_eq!(cli.subcommand_name(parsed.matches), "inspect");
        let selected = cli.subcommand_matches(parsed.matches);
        assert_eq!(
            cli.get_string(selected, "name".into()),
            Some("alice".into())
        );
        assert_eq!(cli.get_int(selected, "count".into()), Some(7));
        assert_eq!(cli.get_float(selected, "ratio".into()), Some(0.5));
        assert_eq!(cli.get_bool(selected, "enabled".into()), Some(false));
        assert_eq!(cli.get_bool(selected, "verbose".into()), Some(true));

        let missing = cli.parse(root, ["origout", "inspect"].map(str::to_owned).to_vec());
        assert_eq!(missing.exit_code, 2);
        assert_eq!(missing.matches, 0);
        assert!(missing.output.contains("required"));

        let invalid = cli.parse(
            root,
            ["origout", "inspect", "alice", "--count", "seven"]
                .map(str::to_owned)
                .to_vec(),
        );
        assert_eq!(invalid.exit_code, 2);
        assert!(invalid.output.contains("invalid value"));
    }

    #[test]
    fn help_and_version_return_display_results() {
        let cli = CliService::default();
        let root = cli.new_command("origout".into());
        cli.set_about(root, "CLI test".into());
        cli.set_version(root, "1.2.3".into());
        let help = cli.parse(root, ["origout", "--help"].map(str::to_owned).to_vec());
        assert_eq!(help.exit_code, 0);
        assert_eq!(help.matches, 0);
        assert!(help.output.contains("Usage: origout"));
        let version = cli.parse(root, ["origout", "--version"].map(str::to_owned).to_vec());
        assert_eq!(version.exit_code, 0);
        assert_eq!(version.output, "origout 1.2.3\n");
    }
}
