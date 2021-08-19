//! This is library part of the rustea implementation.
//!
//! It implements the heavy lifting for the main binary.
pub mod gitea;

use core::fmt;
use faccess::PathExt;
use gitea::{
    gitea_api::{self, ContentEntry, ContentsResponse},
    GiteaClient,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::{self, File},
    io,
    io::{Read, Write},
    path::PathBuf,
};

use crate::gitea::gitea_api::ContentType;

/// A `Result` alias where the `Err` case is `rustea::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type of rustea. It catches either errors from
/// API calls and wraps `gitea_api::Error` or from configuration and
/// file operations.
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
            Error::ApiError(e) => write!(f, "Gitea api error: {}", e),
            Error::IoError(e) => write!(f, "IO Error: {}", e),
            Error::WriteConfiguration(e) => write!(f, "Configuration write error: {}", e),
            Error::ReadConfiguration(e) => write!(f, "Configuration read error: {}", e),
            Error::Push(e) => write!(f, "Error pushing configuration: {}", e),
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

/// The version of rustea
pub const VERSION: &str = "0.1";

/// The default configuration name used by rustea.
pub const DEFAULT_CONF_NAME: &str = ".rustea.toml";

/// The default path is in the users home directory.
pub fn get_default_path() -> Result<String> {
    match env::var_os("HOME") {
        Some(val) => {
            let home = String::from(val.to_str().unwrap());
            Ok(home + "/" + DEFAULT_CONF_NAME)
        }
        None => panic!("Could not find home"),
    }
}

/// The main configuration is serialized by the toml library.
#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub script_folder: String,
    pub repo: RemoteRepository,
}

impl Configuration {
    /// This function tries to read and convert the file provided as `PathBuf` into a new `Configuration`.
    pub fn read_config_file(path: Option<&str>) -> Result<Configuration> {
        let path = PathBuf::from(path.unwrap_or(&get_default_path()?));
        let mut config_string = String::new();
        File::open(path).and_then(|mut file| file.read_to_string(&mut config_string))?;
        Ok(toml::from_str(&config_string)?)
    }

    /// This function writes the `Configuration` to the provided `PathBuf`.
    pub fn write_config_file(&self, file_path: &PathBuf) -> Result<()> {
        // toml::to_string_pretty(self).and_then(|c| write_file(&c, file_path))
        let conf_string = toml::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(conf_string.as_bytes())
            .map_err(Error::IoError)
    }

    /// This function creates a new rustea configuration and stores it
    /// in the users home directory. If no api token is provided, rustea
    /// tries to create a new one by asking the users serveral questions.
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
            repo: RemoteRepository {
                url: client.url,
                api_token: client.api_token,
                repository: client.repository,
                owner: client.username.clone(),
                email: String::new(),
                author: client.username.clone(),
            },
        };

        let path = PathBuf::from(get_default_path()?);
        conf.write_config_file(&path).and(Ok(path))
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Using rustea version {}\nscript_folder={}\nrepo={{\n{}}}",
            VERSION, self.script_folder, self.repo
        )
    }
}

/// This struct defines the access to the remote repository
/// which contains the features sets used by rustea.
#[derive(Debug, Deserialize, Serialize)]
pub struct RemoteRepository {
    pub url: String,
    pub api_token: String,
    pub repository: String,
    pub owner: String,
    pub email: String,
    pub author: String,
}

impl RemoteRepository {
    /// This function constructs a new Gitea API client for requests.
    fn create_api_client(&self) -> Result<GiteaClient> {
        GiteaClient::new(
            &self.url,
            Some(&self.api_token),
            None,
            &self.owner,
            &self.repository,
        )
        .map_err(Error::ApiError)
    }

    /// This function queries the remote repository root and
    /// returns a list of `ContentEntry` with `ContentType::Dir`.
    /// All directories in the root are considered as feature sets.
    fn get_feature_sets(&self, api: &GiteaClient) -> Result<ContentsResponse> {
        api.get_file_or_folder("", Some(ContentType::Dir))
            .map_err(Error::ApiError)
    }

    /// This function returns true if a certain folder in the remote repository root is found.
    fn check_feature_set_exists(&self, api: &GiteaClient, name: &str) -> Result<bool> {
        let content = self.get_feature_sets(&api)?.content;
        for e in content {
            if e.name == name {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// This function prints informations about the remote instance and the
    /// used repository to the command line.
    pub fn info(&self) -> Result<()> {
        let api = self.create_api_client()?;
        let gitea_version = api.get_gitea_version()?;
        let repository = api.get_repository_information()?;
        println!("{}", gitea_version);
        println!("{}", repository);
        Ok(())
    }

    /// This function prints either the feature sets contained in the remote
    /// repository or if `name` is provided all files found in the feature set.
    pub fn list(&self, feature_set: Option<&str>) -> Result<()> {
        let api = self.create_api_client()?;
        match feature_set {
            Some(n) => {
                let feature_set = api.get_folder(n)?;
                println!("Feature Set: {}\n{}", n, feature_set);
            }
            None => {
                let feature_sets = self.get_feature_sets(&api)?;
                println!("Feature Sets:\n{}", feature_sets);
            }
        }
        Ok(())
    }

    /// This function creates a new feature set within the remote repositories root.
    ///
    /// Since git ignores empty folders, a standard way is used. The file empty
    /// `<featurename>/.gitkeep` is created instead.
    /// If the feature already exists nothing is returned and indicates success,
    /// Normaly the API returns the content entry for the created file but this is
    /// useless in this case. We only check the HTTP return code.
    pub fn new_feature_set(&self, feature_set: &str) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, feature_set)? {
            api.create_or_update_file(feature_set, ".gitkeep", "", &self.author, &self.email)?;
            api.create_or_update_file(
                feature_set,
                "scripts/.gitkeep",
                "",
                &self.author,
                &self.email,
            )?;
        }
        Ok(())
    }

    /// This function tries to delete files from the remote repository.
    ///
    /// It takes the `name` of the feature set and optional a `path` to some file.
    /// It no path if provided the whole feature set is deleted. If some path is provided
    /// and `script` is set to true `path` shall point to a file name in the scripts folder
    /// of the feature set. Otherwise the function tries to delete a configuration file
    /// folder denoted by path.
    pub fn delete(
        &self,
        name: &str,
        path: Option<&str>,
        script: bool,
        recursive: bool,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        match path {
            Some(path) if script => api.delete_file_or_folder(
                &format!("{}/scripts/{}", name, path),
                false,
                &self.author,
                &self.email,
            ),
            Some(path) => api.delete_file_or_folder(
                &format!("{}/{}", name, path),
                recursive,
                &self.author,
                &self.email,
            ),
            None => api.delete_file_or_folder(name, true, &self.author, &self.email),
        }
        .map_err(Error::ApiError)
    }

    // fn push_files(
    //     api: &GiteaClient,
    //     feature_set: &str,
    //     files: Vec<PathBuf>,
    //     script: bool,
    // ) -> Vec<String> {
    //     let mut results = vec![];
    //     for file in files {
    //         if file.exists() {
    //             let remote_path = to_remote_path(&file, script)?;
    //             let content = read_file(&file)?;
    //             match api.create_or_update_file(feature_set, &remote_path, &content) {
    //                 Ok(()) => results.push(format!(
    //                     "Successfully uploaded file {} into feature set {}.",
    //                     remote_path, feature_set
    //                 )),
    //                 Err(c) => results.push(format!(
    //                     "Failed to upload file {} int feature set {} with cause {}",
    //                     remote_path, feature_set, c
    //                 )),
    //             }
    //         }
    //     }
    //     results
    // }

    /// This function pushes files to a feature set.
    ///
    /// If no path is provided this function fetches all files stored
    /// in the remote repository and tries to push a local version if found.
    /// Script files are searched in the provided `script_dir`.
    ///
    /// If some path is provided this function push the local file or folder.
    /// Folders are pushed recursively.
    pub fn push(
        &self,
        name: &str,
        script_dir: &str,
        path: Option<&str>,
        script: bool,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, name)? {
            return Err(Error::Push(format!("No features set named {}", name)));
        }

        if let Some(path) = path {
            // Push a config or script file or folder
            let path = PathBuf::from(path).canonicalize()?;
            if path.exists() {
                let files = read_folder(&path)?;

                for entry in files {
                    let remote_path = to_remote_path(&entry, script)?;
                    let content = read_file(&entry)?;
                    let res = api.create_or_update_file(
                        name,
                        &remote_path,
                        &content,
                        &self.author,
                        &self.email,
                    );
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
            let script_remote = format!("{}/scripts/", name);

            for entry in feature_set.content {
                let mut path = entry.path.clone();
                if path.starts_with(&script_remote) {
                    // get the script file names and upload them from the script folder
                    let file_name = path.strip_prefix(&script_remote).unwrap();
                    let mut file_path = PathBuf::from(script_dir);
                    file_path.push(file_name);
                    if file_path.exists() {
                        let content = read_file(&file_path)?;
                        let remote_path = to_remote_path(&file_path, true)?;
                        let res = api.create_or_update_file(
                            name,
                            &remote_path,
                            &content,
                            &self.author,
                            &self.email,
                        );
                        println!(
                            "Uploaded file {} into feature set {} with result {:#?}",
                            remote_path, name, res
                        );
                    }
                } else {
                    // we have a configuration file, so strip feature set and
                    // push file if the path exists, ignore otherwise
                    let file_path = PathBuf::from(path.strip_prefix(&name).unwrap());
                    if file_path.exists() {
                        let content = read_file(&file_path)?;
                        let remote_path = to_remote_path(&file_path, false)?;
                        let res = api.create_or_update_file(
                            name,
                            &remote_path,
                            &content,
                            &self.author,
                            &self.email,
                        );
                        println!(
                            "Uploaded file {} into feature set {} with result {:#?}",
                            remote_path, name, res
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// This function pulls the script file folder from the remote repository.
    ///
    /// It takes the feature set and pulls a found files into the defined local
    /// script directory. It returns an Error if some IO error happens or the
    /// script folder is not writable for the current user.
    fn pull_script_files(
        &self,
        api: &GiteaClient,
        files: &[ContentEntry],
        script: bool,
        script_dir: &str,
    ) -> Result<()> {
        for file in files {
            let content = api.download_file(&file.path)?;
            let path = to_local_path(&file.path, script, script_dir)?;
            // If we have a regular config file, check if the parent folder exists and is writable
            if !script {
                check_folder(&path.parent().unwrap().to_string_lossy())?;
            }
            let mut f = File::create(&path)?;
            f.write_all(content.as_bytes()).map_err(Error::IoError)?;
            println!("Pulled file {}", path.display());
        }
        Ok(())
    }

    pub fn pull(&self, name: &str, script_dir: &str, script: bool, config: bool) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, name)? {
            return Err(Error::Push(format!("No features set named {}", name)));
        }
        let prefix = format!("{}/scripts", name);
        let feature_set = api.get_folder(name)?;

        if script {
            // Pull only the script files
            check_folder(script_dir)?;
            let files = feature_set
                .content
                .into_iter()
                .filter(|e| e.path.starts_with(&prefix))
                .collect::<Vec<ContentEntry>>();
            // let script_files = api.get_folder(&format!("{}/scripts", feature_set))?;
            self.pull_script_files(&api, &files, true, script_dir)?;
        } else if config {
            // Pull only the config files
            let files = feature_set
                .content
                .into_iter()
                .filter(|e| !e.path.starts_with(&prefix))
                .collect::<Vec<ContentEntry>>();
            self.pull_script_files(&api, &files, false, script_dir)?;
            // for file in feature_set.content {
            // for file in files {
            // if !file.path.starts_with(&format!("{}/scripts", name)) {
            // let content = api.download_file(&file.path)?;
            // let path = PathBuf::from(&file.path.strip_prefix(name).unwrap());

            // if !path.parent().unwrap().writable() {
            //     println!("Path {} is not writable", path.display());
            // } else {
            //     let mut f = File::create(&path)?;
            //     match f.write_all(content.as_bytes()) {
            //         Ok(_) => println!("File {} written", path.display()),
            //         Err(e) => println!("Failed with {}", e),
            //     }
            //     // }
            // }
            // }
        } else {
            // Pull everything found in the feature set
            check_folder(script_dir)?;
            for file in feature_set.content {
                if !file.path.starts_with(&format!("{}/scripts", name)) {
                    self.pull_script_files(&api, &[file], false, script_dir)?;
                    // let content = api.download_file(&file.path)?;
                    // let path = PathBuf::from(&file.path.strip_prefix(name).unwrap());

                    // if !path.parent().unwrap().writable() {
                    //     println!("Path {} is not writable", path.display());
                    // } else {
                    //     let mut f = File::create(&path)?;
                    //     match f.write_all(content.as_bytes()) {
                    //         Ok(_) => println!("File {} written", path.display()),
                    //         Err(e) => println!("Failed with {}", e),
                    //     }
                    // }
                } else {
                    // let script_files = api.get_folder(&format!("{}/scripts", feature_set))?;
                    self.pull_script_files(&api, &[file], true, script_dir)?;
                    // self.pull_script_files(&api, name, script_dir)?;
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
            "\turl={}\n\tapi_token={}\n\trepository={}\n\towner={}\n",
            self.url, self.api_token, self.repository, self.owner
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

/// This function converts a `PathBuf` into a remote path.
/// The `path` either corresponds to a script path for a feature set or
/// the path of a configuration file.
fn to_remote_path(path: &PathBuf, script: bool) -> Result<String> {
    match script {
        true => {
            let name = path.file_name().ok_or(Error::Push(format!(
                "{} is not a valid path to a file.",
                path.display()
            )))?;
            Ok(format!("/scripts/{}", name.to_string_lossy()))
        }
        false => Ok(path.to_string_lossy().into_owned()),
    }
}

/// This function converts a remote path from a gitea file into a local file.
/// If the file ist a script file, the file name gets attached to the `script_dir`.
/// Otherwise the whole path without the feature set name is returned.
fn to_local_path(remote_path: &str, script: bool, script_dir: &str) -> Result<PathBuf> {
    let split = match script {
        true => remote_path.rsplit_once("/"),
        false => remote_path.split_once("/"),
    };
    match split {
        Some((_, name)) if script => Ok(PathBuf::from(format!("{}/{}", script_dir, name))),
        Some((_, path)) if !script => Ok(PathBuf::from(format!("/{}", path))),
        None | Some(_) => Err(Error::Push(format!(
            "Remote path {} can not be converted to a local one.",
            remote_path
        ))),
    }
}

/// This function takes a folder path and creates the path if it
/// not exists and checks if the path is writable afterwards.
fn check_folder(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir).canonicalize()?;
    if !path.exists() {
        fs::DirBuilder::new().recursive(true).create(&path)?;
    }
    if !path.writable() {
        return Err(Error::Push(format!(
            "Path {} not writable. Do you need to be root?",
            dir
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{read_file, read_folder, to_local_path, to_remote_path};

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
    fn test_to_remote_path() {
        let path = PathBuf::from(".gitignore");
        let remote_path = to_remote_path(&path, false).unwrap();
        assert_eq!(remote_path, ".gitignore");
        let remote_path = to_remote_path(&path, true).unwrap();
        assert_eq!(remote_path, "scripts/.gitignore");
        let remote_path = to_remote_path(&PathBuf::from("/"), true);
        assert!(remote_path.is_err())
    }

    #[test]
    fn test_to_local_path() {
        let remote_path = "testing/etc/test";
        let local_path = to_local_path(&remote_path, false, "").unwrap();
        assert_eq!(local_path, PathBuf::from("/etc/test"));
        let local_path = to_local_path(&remote_path, true, "/usr/local/bin").unwrap();
        assert_eq!(local_path, PathBuf::from("/usr/local/bin/test"));
        let local_path = to_local_path("test", false, "");
        assert!(local_path.is_err());
    }
}
