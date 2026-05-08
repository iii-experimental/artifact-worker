use artifact_cli::{
    generate_worker, inspect_artifact, plan_worker, verify_worker, ArtifactInput, SourceType,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "artifact-cli-worker")]
#[command(about = "Rust-first artifact-cli iii worker utility")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Inspect {
        #[arg(long)]
        name: String,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long)]
        source: Option<String>,
    },
    Plan {
        #[arg(long)]
        name: String,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long)]
        source: Option<String>,
    },
    Generate {
        #[arg(long)]
        name: String,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        function: Vec<String>,
    },
    Verify {
        #[arg(long)]
        output_dir: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { name, goal, source } => {
            let input = ArtifactInput {
                name,
                goal,
                source_type: Some(SourceType::Docs),
                source,
                functions: vec![],
                output_dir: None,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&inspect_artifact(&input))?
            );
        }
        Command::Plan { name, goal, source } => {
            let input = ArtifactInput {
                name,
                goal,
                source_type: Some(SourceType::Docs),
                source,
                functions: vec![],
                output_dir: None,
            };
            println!("{}", serde_json::to_string_pretty(&plan_worker(&input))?);
        }
        Command::Generate {
            name,
            goal,
            source,
            output_dir,
            function,
        } => {
            let input = ArtifactInput {
                name,
                goal,
                source_type: Some(SourceType::Docs),
                source,
                functions: function,
                output_dir,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&generate_worker(&input)?)?
            );
        }
        Command::Verify { output_dir } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&verify_worker(output_dir)?)?
            );
        }
    }
    Ok(())
}
