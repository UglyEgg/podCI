// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Richard Majewski - Varanid Works

//! Packaging helper: generate man page + shell completions into `./dist/`.
//!
//! This keeps distribution asset generation co-located with the CLI definition
//! without requiring a separate `xtask` workspace crate.

use anyhow::{Context, Result};
use clap::Parser;
use clap_complete::{generate_to, shells};
use clap_mangen::Man;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "podci-assets", arg_required_else_help = true)]
#[command(about = "Generate podCI distribution assets (man page + shell completions)")]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, clap::Subcommand)]
enum Cmd {
    /// Generate man page and shell completions.
    ///
    /// Writes into `./dist/` by default.
    Gen {
        /// Output directory for generated assets.
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    match Args::parse().cmd {
        Cmd::Gen { out_dir } => gen(&out_dir)?,
    }
    Ok(())
}

fn gen(out_dir: &Path) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("create directory {}", out_dir.display()))?;

    let mut cmd = <podci::CliForGen as clap::CommandFactory>::command();
    cmd.set_bin_name("podci");

    // Man page
    let man = Man::new(cmd.clone());
    let man_path = out_dir.join("podci.1");
    {
        let mut buf: Vec<u8> = Vec::new();
        man.render(&mut buf).context("render man page")?;
        fs::write(&man_path, buf).with_context(|| format!("write {}", man_path.display()))?;
    }

    // Completions
    let comp_dir = out_dir.join("completions");
    fs::create_dir_all(&comp_dir)
        .with_context(|| format!("create directory {}", comp_dir.display()))?;
    generate_to(shells::Bash, &mut cmd, "podci", &comp_dir)?;
    generate_to(shells::Zsh, &mut cmd, "podci", &comp_dir)?;
    generate_to(shells::Fish, &mut cmd, "podci", &comp_dir)?;

    println!("generated: {}", out_dir.display());
    Ok(())
}
