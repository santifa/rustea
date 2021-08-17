pub mod gitea;

use core::fmt;
use faccess::PathExt;
use gitea::{gitea_api, GiteaClient};
use serde_derive::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::{self, File},
    io,
    io::{Read, Write},
    path::PathBuf,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ApiError(gitea_api::ApiError),
    IoError(io::Error),
    WriteConfiguration(toml::ser::Error),
    ReadConfiguration(toml::de::Error),
    Push(String),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::ApiError(ref c) => Some(c),
            Error::IoError(ref c) => Some(c),
            Error::WriteConfiguration(_) => None,
            Error::ReadConfiguration(_) => None,
            Error::Push(_) => None,
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            Error::ApiError(ref c) => Some(c),
            Error::IoError(ref c) => Some(c),
            Error::WriteConfiguration(_) => None,
            Error::ReadConfiguration(_) => None,
            Error::Push(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ApiError(_) => write!(f, "Api Error"),
            Error::IoError(_) => write!(f, "Io Error"),
            Error::WriteConfiguration(_) => write!(f, "Config bad"),
            Error::ReadConfiguration(_) => write!(f, "Config bad"),
            Error::Push(s) => write!(f, "{}", s),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<gitea_api::ApiError> for Error {
    fn from(err: gitea_api::ApiError) -> Self {
        Error::ApiError(err)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::WriteConfiguration(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::ReadConfiguration(err)
    }
}

// assume that the configuration file is either in
// in the users home directory or provided on the cli.
/// Name of the default
pub const DEFAULT_CONF_NAME: &str = ".rustea.toml";

/// The default path is in the users home directory
pub fn get_default_path() -> Result<String> {
    match env::var_os("HOME") {
        Some(val) => {
            let home = String::from(val.to_str().unwrap());
            Ok(home + "/" + DEFAULT_CONF_NAME)
        }
        None => panic!("Could not find home"),
    }
}

/// The rustea main configuration
#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub script_folder: String,
    pub email: String,
    pub name: String,
    pub repo: RemoteRepository,
}

impl Configuration {
    /// This function tries to read and convert the file provided as `PathBuf`.
    pub fn read_config_file(path: Option<&str>) -> Result<Configuration> {
        let path = PathBuf::from(path.unwrap_or(&get_default_path()?));
        let mut config_string = String::new();
        File::open(path).and_then(|mut file| file.read_to_string(&mut config_string))?;
        Ok(toml::from_str(&config_string)?)
    }

    /// This function writes the configuration to the provided `PathBuf`.
    pub fn write_config_file(&self, file_path: &PathBuf) -> Result<()> {
        let conf_string = toml::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(conf_string.as_bytes())
            .map_err(Error::IoError)
    }

    /// This function creates a new configuration file called .rustea.toml
    /// in the callers home directory. It either creates a new api key
    /// or uses a provided one for connecting to the api.
    pub fn create_initial_configuration(
        url: &str,
        api_token: Option<&str>,
        token_name: Option<&str>,
        repository: &str,
        owner: &str,
    ) -> Result<PathBuf> {
        let client = GiteaClient::new(url, api_token, token_name, repository, owner)?;

        let conf = Configuration {
            script_folder: "/usr/local/bin".to_owned(),
            email: String::new(),
            name: client.username.to_string(),
            repo: RemoteRepository {
                url: client.url,
                api_token: client.api_token,
                repository: client.repository,
                username: client.username,
            },
        };

        let path = PathBuf::from(get_default_path()?);
        println!("Path {:?}", path);
        conf.write_config_file(&path).and(Ok(path))
    }

    /// Print either the default configuration or from the file provided.
    /// This is just for convience.
    pub fn print_configuration(path: Option<&str>) -> i32 {
        let p = PathBuf::from(path.unwrap_or(&get_default_path().unwrap()));
        if p.exists() && p.is_file() {
            println!(
                "Found configuration {}\n{}",
                p.display(),
                Configuration::read_config_file(p.to_str()).unwrap()
            );
            0
        } else {
            println!("Configuration file not found. Run rustea init --token rustea-devops <repository name> <owner>");
            1
        }
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "script_folder={}\nrepo={{\n{}}}",
            self.script_folder, self.repo
        )
    }
}

#[derive(Deserialize, Serialize)]
pub struct RemoteRepository {
    pub url: String,
    pub api_token: String,
    pub repository: String,
    pub username: String,
}

impl RemoteRepository {
    fn create_api_client(&self) -> GiteaClient {
        let client = GiteaClient::new(
            &self.url,
            Some(&self.api_token),
            None,
            &self.username,
            &self.repository,
        );
        match client {
            Ok(c) => c,
            Err(cause) => panic!("Could not create the Gitea client {}", cause),
        }
    }

    /// Print informations about the remote Gitea instance and the used remote repository.
    pub fn info(&self) -> Result<()> {
        let api = self.create_api_client();
        let gitea_version = api.get_gitea_version()?;
        let repository = api.get_repository_information()?;
        println!("{}", gitea_version);
        println!("{}", repository);
        Ok(())
    }

    /// Print a list of remote available feature sets.
    pub fn list(&self, name: Option<&str>) -> Result<()> {
        let api = self.create_api_client();
        match name {
            Some(n) => {
                let feature_set = api.get_folder(n)?;
                println!("Feature Set: {}\n{}", n, feature_set);
            }
            None => {
                let feature_sets = api.get_repository_features()?;
                println!("Feature Sets:\n{}", feature_sets);
            }
        }
        Ok(())
    }

    /// Create a new feature set
    pub fn new_feature_set(&self, name: &str) {
        let api = self.create_api_client();
        let success = api.create_new_feature_set(name);
        println!("Feature {:?} created with status {:?}", name, success);
    }

    /// Either delete a feature set or a script file or a folder
    pub fn delete_remote(&self, name: &str, path: Option<&str>, script: bool, recursive: bool) {
        let api = self.create_api_client();
        match path {
            Some(path) => {
                let success = if script {
                    // Delete a script file
                    api.delete_script_from_feature_set(name, &path)
                } else {
                    // Delete a configuration file or folder
                    api.delete_conf_from_feature_set(name, &path, recursive)
                };
                println!("Deleted from {} {} with result {:#?}", name, path, success);
            }
            None => {
                // we only delete the feature set
                let success = api.delete_feature_set(name);
                println!("Deleted feature set {} with {:#?}", name, success);
            }
        }
    }

    /// Push files to gitea
    pub fn push(&self, name: &str, path: Option<&str>, script: bool) -> Result<()> {
        let api = self.create_api_client();
        if let Some(path) = path {
            // Push a config or script file or folder
            let path = PathBuf::from(path).canonicalize()?;
            if path.exists() {
                let files = read_folder(&path)?;

                for entry in files {
                    let remote_path = get_remote_path(&entry, script)?;
                    let content = read_file(&entry)?;
                    let res = api.create_or_update_file(name, &remote_path, &content);
                    println!(
                        "Uploaded file {} into feature set {} with result {:#?}",
                        remote_path, name, res
                    );
                }
            } else {
                return Err(Error::Push(format!(
                    "File {} doesn't exists",
                    path.display()
                )));
            }
        } else {
            // Push everything found in the feature set
            let feature_set = api.get_folder(name)?;
            println!("{}", feature_set);
        }
        Ok(())
    }

    pub fn pull(&self, name: &str, script_dir: &str, script: bool, config: bool) -> Result<()> {
        let api = self.create_api_client();
        let feature_set = api.get_folder(name)?;

        if script {
            // Pull only the script files
            let script_path = PathBuf::from(script_dir).canonicalize()?;
            if !script_path.exists() {
                fs::DirBuilder::new().recursive(true).create(&script_path)?;
            }

            if script_path.writable() {
                let script_files = api.get_folder(&format!("{}/scripts", name))?;
                for file in script_files.content {
                    let content = api.download_file(&file.path)?;
                    let mut path = PathBuf::from(&script_path);
                    path.push(file.name);
                    let mut f = File::create(path)?;
                    f.write_all(content.as_bytes()).map_err(Error::IoError)?;
                }
            } else {
                println!(
                    "Could not write to {}. Do you need to be root?",
                    script_path.display()
                );
            }
        } else if config {
            // Pull only the config files
            for file in feature_set.content {
                if !file.path.starts_with(&format!("{}/scripts", name)) {
                    let content = api.download_file(&file.path)?;
                    let path = PathBuf::from(&file.path.strip_prefix(name).unwrap());

                    if !path.parent().unwrap().writable() {
                        println!("Path {} is not writable", path.display());
                    } else {
                        let mut f = File::create(&path)?;
                        match f.write_all(content.as_bytes()) {
                            Ok(_) => println!("File {} written", path.display()),
                            Err(e) => println!("Failed with {}", e),
                        }
                    }
                }
            }
        } else {
            // Pull everything found in the feature set
            for file in feature_set.content {
                if !file.path.starts_with(&format!("{}/scripts", name)) {
                    let content = api.download_file(&file.path)?;
                    let path = PathBuf::from(&file.path.strip_prefix(name).unwrap());

                    if !path.parent().unwrap().writable() {
                        println!("Path {} is not writable", path.display());
                    } else {
                        let mut f = File::create(&path)?;
                        match f.write_all(content.as_bytes()) {
                            Ok(_) => println!("File {} written", path.display()),
                            Err(e) => println!("Failed with {}", e),
                        }
                    }
                } else {
                    let script_path = PathBuf::from(script_dir).canonicalize()?;
                    if !script_path.exists() {
                        fs::DirBuilder::new().recursive(true).create(&script_path)?;
                    }

                    if script_path.writable() {
                        let script_files = api.get_folder(&format!("{}/scripts", name))?;
                        for file in script_files.content {
                            let content = api.download_file(&file.path)?;
                            let mut path = PathBuf::from(&script_path);
                            path.push(file.name);
                            let mut f = File::create(path)?;
                            f.write_all(content.as_bytes()).map_err(Error::IoError)?;
                        }
                    } else {
                        println!(
                            "Could not write to {}. Do you need to be root?",
                            script_path.display()
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

impl Display for RemoteRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\turl={}\n\tapi_token={}\n\trepository={}\n\tusername={}\n",
            self.url, self.api_token, self.repository, self.username
        )
    }
}

/// Read a file denoted by a `PathBuf` into a `String` or return the io Error.
fn read_file(path: &PathBuf) -> Result<String> {
    let mut string = String::new();
    File::open(path.as_path()).and_then(|mut f| f.read_to_string(&mut string))?;
    Ok(string)
}

/// This function takes a path and either returns it directly as vector if
/// the path denotes a singles file. Otherwise, the directory is crawled
/// recursively and a vector of all known files below `Path` is returned.
fn read_folder(path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = vec![];
    let path = path.canonicalize()?;
    if path.is_dir() {
        // Check if the original path is a folder
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                // Recursively push folders
                let mut entries = read_folder(&entry.path())?;
                v.append(&mut entries);
            } else {
                // Push a single file
                v.push(entry.path().canonicalize()?)
            }
        }
    } else {
        v.push(path);
    }
    Ok(v)
}

/// This function constructs the remote path for a `PathBuf` as a string.
/// If `script` is set to true only the filename is returned otherwise
/// the whole path is returned.
fn get_remote_path(p: &PathBuf, script: bool) -> Result<String> {
    match script {
        true => p
            .file_name()
            .ok_or(Error::Push(format!(
                "Failure converting path {} to string",
                p.display()
            )))
            .and_then(|s| Ok(String::from(s.to_string_lossy()))),
        false => Ok(String::from(p.to_string_lossy())),
    }
}

/// This function simply writes content to a `PathBuf`.
fn write_file(content: &str, path: &PathBuf) -> Result<()> {
    let mut file = File::create(path)?;
    Ok(file.write_all(content.as_bytes())?)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{get_remote_path, read_file, read_folder, write_file};

    #[test]
    fn test_read_folder() {
        let path = PathBuf::from("./tests");
        let res = read_folder(&path);
        assert!(res.is_ok());
    }

    #[test]
    fn test_read_folder_single_file() {
        let path = PathBuf::from("./tests/test_config.rs");
        let res = read_folder(&path);
        assert!(res.is_ok());
    }

    #[test]
    fn test_read_folder_recursively() {
        let path = PathBuf::from("./src");
        let res = read_folder(&path);
        assert!(res.is_ok());
    }

    #[test]
    fn test_read_file() {
        let path = PathBuf::from(".gitignore");
        let res = read_file(&path);
        assert!(res.is_ok())
    }

    #[test]
    fn test_get_remote_path() {
        let path = PathBuf::from(".gitignore");
        let res = get_remote_path(&path, false);
        assert!(res.is_ok());
        let res = get_remote_path(&path, true);
        assert!(res.is_ok());
    }

    #[test]
    fn test_write_file() {
        let path = PathBuf::from("test_bin/testfile");
        let content = r#"testing\n"#;
        let res = write_file(content, &path);
        assert!(res.is_ok())
    }
}
