mod config;
mod state;
mod wallpaper;

use clap::{Parser, Subcommand};
use config::Config;
use state::State;

#[derive(Parser)]
#[command(name = "wproulette", about = "Wallpaper roulette — browse, star, trash, and rotate wallpapers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Pick a random wallpaper
    Random,
    /// Pick a random wallpaper from starred only
    Starred,
    /// Toggle star on current wallpaper
    Star,
    /// Trash current wallpaper and pick a new one
    Trash,
    /// Restore last trashed wallpaper to its original location
    Restore,
    /// Show current status as JSON (for waybar)
    Status {
        /// Which status to show
        #[arg(value_enum)]
        module: StatusModule,
    },
    /// Show current wallpaper path
    Current,
    /// List starred wallpapers
    ListStarred,
    /// List recently trashed wallpapers
    ListTrashed {
        /// Number of entries to show
        #[arg(short, long, default_value = "5")]
        n: usize,
    },
    /// Show config
    Config,
}

#[derive(Clone, clap::ValueEnum)]
enum StatusModule {
    /// Star button status
    Star,
    /// Trash button status
    Trash,
}

fn main() {
    let cli = Cli::parse();
    let config = Config::load();
    let state = State::new(&config.wallpaper_dir);

    match cli.command {
        Commands::Random => cmd_random(&config, &state, false),
        Commands::Starred => cmd_random(&config, &state, true),
        Commands::Star => cmd_star(&config, &state),
        Commands::Trash => cmd_trash(&config, &state),
        Commands::Restore => cmd_restore(&state),
        Commands::Status { module } => cmd_status(&config, &state, module),
        Commands::Current => cmd_current(&state),
        Commands::ListStarred => cmd_list_starred(&state),
        Commands::ListTrashed { n } => cmd_list_trashed(&state, n),
        Commands::Config => cmd_config(&config),
    }
}

fn cmd_random(config: &Config, state: &State, starred_only: bool) {
    match wallpaper::pick_random(config, state, starred_only) {
        Some(path) => {
            if let Err(e) = wallpaper::apply(&path, config) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            state.set_current(&path);
        }
        None => {
            eprintln!("No wallpapers found");
            std::process::exit(1);
        }
    }
}

fn cmd_star(_config: &Config, state: &State) {
    let Some(current) = state.current() else {
        eprintln!("No current wallpaper");
        std::process::exit(1);
    };
    let is_starred = state.toggle_star(&current);
    if is_starred {
        println!("Starred: {}", current.display());
    } else {
        println!("Unstarred: {}", current.display());
    }
}

fn cmd_trash(config: &Config, state: &State) {
    let Some(current) = state.current() else {
        eprintln!("No current wallpaper");
        std::process::exit(1);
    };
    if state.is_starred(&current) {
        eprintln!("Cannot trash starred wallpaper — unstar first");
        std::process::exit(1);
    }
    if let Err(e) = state.trash(&current) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    // Pick a new one
    cmd_random(config, state, false);
}

fn cmd_restore(state: &State) {
    match state.restore_last() {
        Ok(path) => println!("Restored: {}", path.display()),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_status(config: &Config, state: &State, module: StatusModule) {
    let current = state.current();

    match module {
        StatusModule::Star => {
            let is_starred = current.as_ref().is_some_and(|p| state.is_starred(p));
            let (icon, class, tooltip) = if is_starred {
                (&config.icons.star_active, "starred", "Starred")
            } else {
                (&config.icons.star_inactive, "unstarred", "Not starred")
            };
            println!(
                r#"{{"text": "{}", "tooltip": "{}", "class": "{}"}}"#,
                icon, tooltip, class
            );
        }
        StatusModule::Trash => {
            let is_starred = current.as_ref().is_some_and(|p| state.is_starred(p));
            let (icon, tooltip, class) = if is_starred {
                (&config.icons.trash_inactive, "Unstar first to trash", "disabled")
            } else {
                (&config.icons.trash_active, "Trash wallpaper", "enabled")
            };
            println!(
                r#"{{"text": "{}", "tooltip": "{}", "class": "{}"}}"#,
                icon, tooltip, class
            );
        }
    }
}

fn cmd_current(state: &State) {
    match state.current() {
        Some(path) => println!("{}", path.display()),
        None => {
            eprintln!("No current wallpaper");
            std::process::exit(1);
        }
    }
}

fn cmd_list_starred(state: &State) {
    for path in state.starred() {
        println!("{}", path.display());
    }
}

fn cmd_list_trashed(state: &State, n: usize) {
    let entries = state.trashed_entries(n);
    if entries.is_empty() {
        println!("No trashed wallpapers");
        return;
    }
    for (trash_path, original_path) in &entries {
        println!("{} <- {}", original_path.display(), trash_path.display());
    }
}

fn cmd_config(config: &Config) {
    println!("{}", toml::to_string_pretty(config).unwrap());
}
