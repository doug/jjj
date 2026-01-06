use crate::cli::{Cli, Shell};
use crate::error::Result;
use clap::CommandFactory;
use clap_complete::{generate, shells};
use std::io;

pub fn execute(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = "jjj";

    match shell {
        Shell::Bash => {
            generate(shells::Bash, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Zsh => {
            generate(shells::Zsh, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Fish => {
            generate(shells::Fish, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::PowerShell => {
            generate(shells::PowerShell, &mut cmd, bin_name, &mut io::stdout());
        }
        Shell::Elvish => {
            generate(shells::Elvish, &mut cmd, bin_name, &mut io::stdout());
        }
    }

    Ok(())
}
