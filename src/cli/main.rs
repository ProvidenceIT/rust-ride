//! RustRide CLI - Command-line interface for headless daemon control.
//!
//! This binary communicates with the RustRide daemon via Unix domain sockets
//! to control rides, workouts, sensors, and more.
//!
//! Note: This binary is only functional on Linux. On other platforms,
//! it will print an error message and exit.
//!
//! T035: CLI daemon commands connected to IPC client

#[cfg(target_os = "linux")]
mod linux_main {
    use clap::{Parser, Subcommand};

    use rustride::cli::commands::daemon::{self, DaemonCommands};
    use rustride::cli::commands::ride::{self, RideCommands};
    use rustride::cli::commands::rides::{self, RidesCommands};
    use rustride::cli::commands::sensors::{self, SensorsCommands};
    use rustride::cli::commands::workout::{self, WorkoutCommands};
    use rustride::cli::set_json_output;

    /// RustRide CLI - Control the RustRide daemon
    #[derive(Debug, Parser)]
    #[command(name = "rustride-cli")]
    #[command(author, version, about, long_about = None)]
    pub struct Cli {
        /// Output in JSON format for scripting
        #[arg(long, global = true)]
        json: bool,

        #[command(subcommand)]
        command: Commands,
    }

    /// Top-level commands
    #[derive(Debug, Subcommand)]
    pub enum Commands {
        /// Daemon control commands
        #[command(subcommand)]
        Daemon(DaemonCommands),

        /// Ride control commands
        #[command(subcommand)]
        Ride(RideCommands),

        /// Rides management (list, export)
        #[command(subcommand)]
        Rides(RidesCommands),

        /// Sensor management commands
        #[command(subcommand)]
        Sensors(SensorsCommands),

        /// Workout control commands
        #[command(subcommand)]
        Workout(WorkoutCommands),

        /// Show daemon and session status (shorthand for 'daemon status')
        Status,
    }

    pub fn run() -> i32 {
        let cli = Cli::parse();

        // Set JSON output mode
        if cli.json {
            set_json_output(true);
        }

        // Create tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

        rt.block_on(async {
            match cli.command {
                Commands::Daemon(cmd) => daemon::execute(cmd).await,
                Commands::Ride(cmd) => ride::execute(cmd).await,
                Commands::Rides(cmd) => rides::execute(cmd).await,
                Commands::Sensors(cmd) => sensors::execute(cmd).await,
                Commands::Workout(cmd) => workout::execute(cmd).await,
                Commands::Status => daemon::execute(DaemonCommands::Status).await,
            }
        })
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        let exit_code = linux_main::run();
        std::process::exit(exit_code);
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("Error: rustride-cli is only supported on Linux.");
        eprintln!("The headless/daemon mode requires Linux-specific features.");
        std::process::exit(1);
    }
}
