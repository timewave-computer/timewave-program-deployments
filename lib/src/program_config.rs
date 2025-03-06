use valence_program_manager::program_config::ProgramConfig;

pub(crate) fn read_program_config_from_json(path: &str) -> ProgramConfig {
    let content = std::fs::read_to_string(path).expect("Unable to open program config file");
    serde_json::from_str::<ProgramConfig>(&content).expect("Failed to parse into ProgramConfig")
}
