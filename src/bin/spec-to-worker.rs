use clap::{Parser, Subcommand};
use spec_to_worker::{init_options, register_spec_to_worker_primitives};

#[derive(Debug, Parser)]
#[command(name = "spec-to-worker")]
#[command(about = "Spec-to-worker iii worker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve {
        #[arg(long, env = "III_URL", default_value = "ws://localhost:49134")]
        iii_url: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { iii_url } => {
            let iii = iii_sdk::register_worker(&iii_url, init_options());
            let refs = register_spec_to_worker_primitives(&iii);
            eprintln!(
                "spec-to-worker registered {} spec-to-worker::* iii functions against {}",
                refs.len(),
                iii_url
            );
            std::thread::park();
            iii.shutdown();
        }
    }
    Ok(())
}
