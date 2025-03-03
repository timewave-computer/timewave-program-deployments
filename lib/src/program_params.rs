use config::Config as ConfigHelper;
use std::{collections::HashMap, error::Error, path::PathBuf};

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

    let params = match ConfigHelper::builder()
        .add_source(config::File::with_name(&format!(
            "{}/{}.toml",
            params_path_str, env
        )))
        .build()
    {
        Ok(cfg) => cfg
            .try_deserialize::<HashMap<String, String>>()
            .map_err(|e| format!("Failed to parse program params : {}", e)),
        Err(_) => Err("Failed to parse program params".to_string()),
    }?;

    Ok(ProgramParams(params))
}

#[derive(Debug)]
pub struct ProgramParams(HashMap<String, String>);

impl ProgramParams {
    pub fn get(&self, key: &str) -> String {
        self.0
            .get(key)
            .expect(format!("Key {} not found", key).as_str())
            .to_string()
    }
}
