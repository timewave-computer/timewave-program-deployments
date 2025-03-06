use config::Config as ConfigHelper;
use std::{error::Error, path::PathBuf};

pub fn get_program_params(
    program_path: &PathBuf,
    env: &str,
) -> Result<ProgramParams, Box<dyn Error>> {
    let params_path = std::env::current_dir()?
        .join(program_path)
        .join("program_params");
    let params_path_str = params_path
        .to_str()
        .expect("Params path should be a string");
    let params_env_path = params_path.join(format!("{}.toml", env));

    if !params_env_path.exists() {
        return Err(format!("Program params file not found: {}.toml", env).into());
    }

    let params = ConfigHelper::builder()
        .add_source(config::File::with_name(&format!(
            "{}/{}.toml",
            params_path_str, env
        )))
        .build()?;

    Ok(ProgramParams::new(params))
}

#[derive(Debug)]
pub struct ProgramParams {
    cfg: ConfigHelper,
}

impl ProgramParams {
    pub fn new(cfg: ConfigHelper) -> Self {
        ProgramParams { cfg }
    }

    pub fn get(&self, key: &str) -> String {
        self.cfg
            .get::<String>(key)
            .unwrap_or_else(|_| panic!("Key {} not found", key))
    }

    pub fn get_array(&self, key: &str) -> Vec<String> {
        self.cfg
            .get_array(key)
            .unwrap_or_else(|_| panic!("Key {} not found", key))
            .iter()
            .map(|v| v.to_string())
            .collect()
    }
}
