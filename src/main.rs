use clap::Parser;
use jjj::cli;
use jjj::commands;

fn main() {
    let cli = cli::Cli::parse();
    // Opt-in, `JJJ_TELEMETRY=1` only; a single `getenv` otherwise. Timed here
    // rather than inside the dispatcher so the recorded `ms` is the whole
    // invocation, and so a command that exits non-zero is still counted.
    let probe = jjj::telemetry::start();
    let result = commands::execute(cli);
    jjj::telemetry::finish(probe);
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(e.exit_code());
    }
}
