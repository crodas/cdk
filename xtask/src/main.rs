//! Workspace automation. Run with `cargo xtask <command>`.

mod nitro;
mod shell;

use clap::{Parser, Subcommand};

use crate::nitro::NitroTask;

/// Repository tasks.
#[derive(Debug, Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Regenerate the React Native Nitro bindings from the UniFFI metadata.
    #[command(alias = "react-native")]
    Bindings {
        /// Skip the nitrogen step and the adapters that depend on it.
        #[arg(long)]
        spec_only: bool,
    },
    /// Build the Rust library for the iOS targets and assemble an XCFramework.
    Ios {
        /// Build the debug profile instead of release.
        #[arg(long)]
        debug: bool,
    },
    /// Build the Rust library for the Android ABIs into `jniLibs`.
    Android {
        /// Build the debug profile instead of release.
        #[arg(long)]
        debug: bool,
    },
    /// Build and run the C++ harness over the generated bridge.
    TestNitro,
    /// Type-check the generated Nitro adapters against the Nitro and JSI headers.
    CheckNitro,
    /// Run the Node harness tests against the generated koffi binding.
    TestNode,
    /// Run the TypeScript versus native benchmark in Node.
    BenchNitro,
}

fn main() -> std::process::ExitCode {
    let result = match Cli::parse().command {
        Command::Bindings { spec_only } => NitroTask::new().and_then(|task| task.bindings(spec_only)),
        Command::Ios { debug } => NitroTask::new().and_then(|task| task.ios(!debug)),
        Command::Android { debug } => NitroTask::new().and_then(|task| task.android(!debug)),
        Command::TestNitro => NitroTask::new().and_then(|task| task.test_cpp()),
        Command::CheckNitro => NitroTask::new().and_then(|task| task.check_cpp()),
        Command::TestNode => NitroTask::new().and_then(|task| task.test_node()),
        Command::BenchNitro => NitroTask::new().and_then(|task| task.bench()),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
