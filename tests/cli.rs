use std::process::{Command, Output};

fn run(args: &[&str], config_dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_origout"))
        .args(args)
        .env("XDG_CONFIG_HOME", config_dir)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap()
}

#[test]
fn checkmate_cli_entry_handles_identity_and_parser_output() {
    let config_dir = std::env::temp_dir().join(format!(
        "origout-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&config_dir).unwrap();

    let help = run(&["--help"], &config_dir);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("identity"));

    let version = run(&["--version"], &config_dir);
    assert!(version.status.success());
    assert_eq!(version.stdout, b"origout 0.1.0\n");

    let invalid = run(&["identity", "--unknown"], &config_dir);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("unexpected argument"));

    let identity = run(&["identity"], &config_dir);
    assert!(identity.status.success());
    let public = String::from_utf8(identity.stdout).unwrap();
    let raw = run(&["identity", "--raw"], &config_dir);
    assert!(raw.status.success());
    let raw = String::from_utf8(raw.stdout).unwrap();
    assert_eq!(public, format!("Current public key: {raw}"));
    assert_eq!(raw.trim().len(), 64);

    std::fs::remove_dir_all(config_dir).unwrap();
}
