mod test_helpers;

use std::process::Command;
use test_helpers::jj_available;

/// Helper to run jjj without needing a repo (completion doesn't need one)
fn run_jjj_raw(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_jjj"))
        .args(args)
        .output()
        .expect("Failed to run jjj command")
}

#[test]
fn test_completion_bash_outputs_script() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "bash"]);
    assert!(
        output.status.success(),
        "completion bash should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Bash completion output should not be empty"
    );
    // Bash completions typically contain _jjj or complete -F
    assert!(
        stdout.contains("_jjj") || stdout.contains("complete"),
        "Bash completion should contain function name or complete directive. Got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_completion_zsh_outputs_script() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "zsh"]);
    assert!(
        output.status.success(),
        "completion zsh should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Zsh completion output should not be empty"
    );
    // Zsh completions typically contain #compdef or _jjj
    assert!(
        stdout.contains("#compdef") || stdout.contains("_jjj"),
        "Zsh completion should contain #compdef or _jjj. Got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_completion_fish_outputs_script() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "fish"]);
    assert!(
        output.status.success(),
        "completion fish should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Fish completion output should not be empty"
    );
    // Fish completions use the `complete` command
    assert!(
        stdout.contains("complete") && stdout.contains("jjj"),
        "Fish completion should contain 'complete' and 'jjj'. Got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_completion_powershell_outputs_script() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "power-shell"]);
    assert!(
        output.status.success(),
        "completion powershell should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "PowerShell completion output should not be empty"
    );
    // PowerShell completions typically contain Register-ArgumentCompleter
    assert!(
        stdout.contains("Register-ArgumentCompleter") || stdout.contains("jjj"),
        "PowerShell completion should contain Register-ArgumentCompleter or jjj. Got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_completion_elvish_outputs_script() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "elvish"]);
    assert!(
        output.status.success(),
        "completion elvish should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Elvish completion output should not be empty"
    );
    // Elvish completions reference the binary name
    assert!(
        stdout.contains("jjj"),
        "Elvish completion should reference jjj. Got: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_completion_scripts_contain_subcommands() {
    if !jj_available() {
        return;
    }

    // Verify that completion scripts include known subcommands
    for shell in &["bash", "zsh", "fish"] {
        let output = run_jjj_raw(&["completion", shell]);
        assert!(
            output.status.success(),
            "completion {} should succeed",
            shell
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        // All shells should reference key subcommands in their completions
        assert!(
            stdout.contains("problem"),
            "{} completion should reference 'problem' subcommand. Got: {}",
            shell,
            &stdout[..stdout.len().min(1000)]
        );
        assert!(
            stdout.contains("solution"),
            "{} completion should reference 'solution' subcommand. Got: {}",
            shell,
            &stdout[..stdout.len().min(1000)]
        );
        assert!(
            stdout.contains("fetch"),
            "{} completion should reference 'fetch' subcommand. Got: {}",
            shell,
            &stdout[..stdout.len().min(1000)]
        );
        assert!(
            stdout.contains("push"),
            "{} completion should reference 'push' subcommand. Got: {}",
            shell,
            &stdout[..stdout.len().min(1000)]
        );
    }
}

#[test]
fn test_completion_invalid_shell_fails() {
    if !jj_available() {
        return;
    }

    let output = run_jjj_raw(&["completion", "invalid-shell"]);
    assert!(
        !output.status.success(),
        "completion with invalid shell should fail"
    );
}
