use std::path::Path;

/// Loads a dotenv file, gnoring potential failure.
///
/// It loads a single `.env` file from the directory passed as argument.
pub fn load_dotenv_from_dir(env_dir: &Path) -> Result<(), dotenvy::Error> {
    let load = |p: &Path| {
        let file_path = p.join(".env");
        dotenvy::from_path(file_path)?;
        Ok(())
    };
    load(env_dir)
}
