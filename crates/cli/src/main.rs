// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = podci::CliForGen::parse();
    if !cli.about && cli.command.is_none() {
        let mut cmd = podci::CliForGen::command();
        let _ = cmd.print_help();
        eprintln!();
        return ExitCode::from(2);
    }
    match real_main(cli.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // Ensure errors always print, even if tracing isn't configured.
            eprintln!("error: {err:#}");
            // Surface operator hints for common Podman failures in human mode.
            if cli.log_format == "human" {
                if let Some(hints) = podci::operator_hints_for_error(&err) {
                    eprintln!();
                    eprintln!("hint: {hints}");
                }
            }
            ExitCode::from(1)
        }
    }
}

fn real_main(cli: podci::CliForGen) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(podci::run_cli(cli))
}
