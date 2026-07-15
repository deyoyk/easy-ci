use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "eci", about = "Internal CI/CD tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    Create,
    List,
    Delete { name: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => println!("TODO: init"),
        Commands::Project { action } => match action {
            ProjectAction::Create => println!("TODO: project create"),
            ProjectAction::List => println!("TODO: project list"),
            ProjectAction::Delete { name } => println!("TODO: project delete {}", name),
        },
    }
}
