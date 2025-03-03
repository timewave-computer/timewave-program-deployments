mod helpers;
mod manager_config;
mod program_config;
mod program_params;

use std::{ error::Error, io::Write, path::PathBuf};

use chrono::Utc;
use clap::{command, Parser};
use dotenvy::dotenv;
use helpers::verify_path;
use manager_config::set_manager_config;
use program_config::read_program_config_from_json;
use program_params::get_program_params;
use valence_program_manager::program_config::ProgramConfig;

// Reexport params to programs
pub use program_params::ProgramParams;

// |X| - Read or get the manager config
// |X| - read program parameters into a map
// |X| - helper function to read a parameter from the map
// |X| - call the program builder with the parameters
// deploy the program using the manager
// save program json files (Raw and instantiated)

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enviroment config to use Ex: mainnet, testnet, local
    #[arg(short, long, default_value = "mainnet")]
    target_env: String,
    /// Absolute path to the program config json file
    #[arg(short, long)]
    program_config_path: Option<String>,
}

pub async fn main<F>(program_path: &str, builder: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(ProgramParams) -> ProgramConfig,
{
    // Load .env file environment variables
    dotenv().expect(".env file not found");

    let args = Args::parse();
    let timestamp = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();

    // Get and verify paths
    let curr_dir = std::env::current_dir()?;
    let program_path = curr_dir.join(
        PathBuf::from(program_path)
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    );

    verify_path(program_path.clone())?;

    // Set manager config for the chosen environment
    set_manager_config(&args.target_env).await?;
    
    // If a path to program_config.json was passed, use it
    let mut program_config = if let Some(program_config_path) = args.program_config_path {
        read_program_config_from_json(&program_config_path)
    } else {
        // Else build the program config from the builder
        let program_params = get_program_params(&program_path, &args.target_env)?;

        builder(program_params)
    };

    // Write the raw program config to file
    write_to_output(program_config.clone(), &program_path, &timestamp, "raw")?;

    // Use program manager to deploy the program
    valence_program_manager::init_program(&mut program_config).await?;

    // Write instantiated program to file
    write_to_output(program_config, &program_path, &timestamp, "instantiated")?;

    Ok(())
}

fn write_to_output(
    program_config: ProgramConfig,
    program_path: &PathBuf,
    time: &str,
    prefix: &str,
) -> Result<(), Box<dyn Error>> {
    let path = program_path.join("output").join(time);

    if !path.exists() {
        std::fs::create_dir_all(path.clone())?;
    }

    // Construct the full file path
    let file_name = format!("{}-program-config.json", prefix);
    let file_path = path.join(file_name.clone());

    // Create and write to the file
    let mut file = std::fs::File::create(file_path.clone())?;

    // Serialize the data to a string
    let content = serde_json::to_string(&program_config)?;

    file.write_all(content.as_bytes())?;

    Ok(())
}
