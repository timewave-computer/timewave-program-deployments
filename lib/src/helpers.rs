use std::{error::Error, path::PathBuf};

// Verify the program path exists and everything was called from the right place
pub(crate) fn verify_path(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let curr_dir = std::env::current_dir()?;

    // Verify we have a "programs" directory
    let programs_dir = curr_dir.join("programs");

    if !programs_dir.exists() {
        return Err("Programs path doesn't exists, make sure you ran the script from the workplace directory".into());
    }

    // Verify program directory exists
    let program_path = curr_dir.join(path);

    if !program_path.exists() {
        return Err("Program does not exist".into());
    }

    Ok(())
}
