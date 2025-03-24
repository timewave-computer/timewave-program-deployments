use config::Config as ConfigHelper;
use std::{error::Error, path::Path};

const MANAGER_CONFIGS_REPO_URL: &str =
    "https://github.com/timewave-computer/valence-program-manager-config.git";

pub fn get_manager_config(
    path: &str,
) -> Result<valence_program_manager::config::Config, Box<dyn Error>> {
    // TODO: Get config from our repo if env exists
    let path = &path.to_lowercase();
    let config_path = std::env::current_dir()?.join("manager_configs").join(path);
    let config_path_str = config_path
        .to_str()
        .expect("Config path should be a string");

    if !config_path.exists() {
        clone_config_from_repo(path, &config_path)?;
    }

    if let Ok(cfg) = ConfigHelper::builder()
        .add_source(config::File::with_name(&format!(
            "{}/config.json",
            config_path_str
        )))
        .build()
    {
        return cfg.try_deserialize().map_err(|e| e.into());
    };

    ConfigHelper::builder()
        .add_source(
            glob::glob(&format!("{}/*", config_path_str))
                .unwrap()
                .filter_map(|path| {
                    let p = path.unwrap();

                    if p.is_dir() {
                        None
                    } else {
                        Some(config::File::from(p))
                    }
                })
                .collect::<Vec<_>>(),
        )
        .add_source(
            glob::glob(&format!("{}/**/*", config_path_str))
                .unwrap()
                .filter_map(|path| {
                    let p = path.unwrap();
                    if p.is_dir() {
                        None
                    } else {
                        Some(config::File::from(p))
                    }
                })
                .collect::<Vec<_>>(),
        )
        .build()?
        .try_deserialize()
        .map_err(|e| e.into())
    // .map_err(|_| "Failed to parse config".into())
}

pub(crate) async fn set_manager_config(path: &str) -> Result<(), Box<dyn Error>> {
    // Read the config
    let config = get_manager_config(path)?;

    // Set the global config of the manager with the read config
    let mut gc = valence_program_manager::config::GLOBAL_CONFIG.lock().await;
    *gc = config;
    Ok(())
}

fn clone_config_from_repo(env_path: &str, config_path: &Path) -> Result<(), Box<dyn Error>> {
    // DO NOT CHANGE THIS
    let tmp_dir = std::env::current_dir()?.join("tmp");

    if !tmp_dir.exists() {
        std::fs::create_dir_all(&tmp_dir)?;
    }

    let tmp_configs = tmp_dir.join("manager_configs");

    cmd_lib::run_cmd!(git clone ${MANAGER_CONFIGS_REPO_URL} ${tmp_configs})?;

    let tmp_config_path = tmp_configs.join(env_path);

    if !tmp_config_path.exists() {
        return Err(format!("Manager config for {} environment does not exist", env_path).into());
    }

    cmd_lib::run_cmd!(
        mv ${tmp_config_path} ${config_path};
        rm -rf ${tmp_dir};
    )?;

    Ok(())
}
