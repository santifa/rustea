//! This is library part of the rustea implementation.
//!
//! It implements the heavy lifting for the main binary.

pub mod error;
pub mod gitea;
pub mod updater;
/// rustea is a small cli tool to interact with git repositories hosted
/// by Gitea Instances. Copyright (C) 2021  Henrik Jürges (juerges.henrik@gmail.com)
///
/// This program is free software: you can redistribute it and/or modify
/// it under the terms of the GNU General Public License as published by
/// the Free Software Foundation, either version 3 of the License, or
/// (at your option) any later version.
///
/// This program is distributed in the hope that it will be useful,
/// but WITHOUT ANY WARRANTY; without even the implied warranty of
/// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
/// GNU General Public License for more details.
///
/// You should have received a copy of the GNU General Public License
/// along with this program. If not, see <https://www.gnu.org/licenses/>.
use core::fmt;
use error::{Error, Result};
use gitea::{
    gitea_api::{ContentEntry, ContentType, ContentsResponse},
    GiteaClient,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
};
use tabwriter::TabWriter;

/// The version of rustea
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The default configuration name used by rustea.
const DEFAULT_CONF_NAME: &str = ".rustea.toml";

/// The default path is in the users home directory.
fn get_default_path() -> Result<String> {
    match env::var_os("HOME") {
        Some(val) => {
            let home = String::from(val.to_str().unwrap());
            Ok(home + "/" + DEFAULT_CONF_NAME)
        }
        None => Err(Error::Configuration(error::ConfigError::LocationError)),
    }
}

/// The main configuration is serialized by the toml library.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RusteaConfiguration {
    script_folder: PathBuf,
    exclude: String,
    repo: RepositoryConfig,
}

impl Display for RusteaConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rustea version {}\nscript_folder = {}\nexclude= {}\nrepo = {{\n{}\n}}",
            VERSION,
            self.script_folder.canonicalize().unwrap().display(),
            self.exclude,
            self.repo
        )
    }
}

impl RusteaConfiguration {
    /// This function tries to read and convert the file provided as `PathBuf` into a new `Configuration`.
    pub fn read_config_file(path: Option<&str>) -> Result<RusteaConfiguration> {
        let path = PathBuf::from(path.unwrap_or(&get_default_path()?));
        let mut config_string = String::new();
        File::open(path).and_then(|mut file| file.read_to_string(&mut config_string))?;
        Ok(toml::from_str(&config_string)?)
    }

    /// This function writes the `Configuration` to the provided `PathBuf`.
    pub fn write_config_file(&self, file_path: &Path) -> Result<()> {
        // toml::to_string_pretty(self).and_then(|c| write_file(&c, file_path))
        let conf_string = toml::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(conf_string.as_bytes()).map_err(Error::Io)
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
        let conf = RusteaConfiguration {
            script_folder: PathBuf::from("/usr/local/bin"),
            exclude: ".git".to_owned(),
            repo: RepositoryConfig {
                url: client.url,
                api_token: client.api_token,
                repository: client.repository,
                owner: client.owner.clone(),
                email: String::new(),
                author: client.owner,
            },
        };

        let path = PathBuf::from(get_default_path()?);
        conf.write_config_file(&path).and(Ok(path))
    }
}

/// This struct defines the access to the remote repository
/// which contains the features sets used by rustea.
#[derive(Debug, Default, Deserialize, Serialize)]
struct RepositoryConfig {
    url: String,
    api_token: String,
    repository: String,
    owner: String,
    email: String,
    author: String,
}

impl Display for RepositoryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tw = TabWriter::new(vec![]);

        write!(
            &mut tw,
            "\turl\t= {}
             \tapi_token\t= {}
             \trepository\t= {}
             \towner\t= {}
             \temail\t= {}
             \tauthor\t= {}",
            self.url, self.api_token, self.repository, self.owner, self.email, self.author
        )
        .unwrap();

        match tw.into_inner() {
            Ok(w) => write!(f, "{}", String::from_utf8_lossy(&w)),
            Err(e) => write!(f, "Failed to align config: {}", e),
        }
    }
}

/// The `RemoteRepository` deals with the actual backend repository
/// and handles all the actions that can take place.
pub struct RemoteRepository {
    config: RusteaConfiguration,
    api: GiteaClient,
}

impl Display for RemoteRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = match self.info() {
            Ok(c) => c,
            Err(e) => format!("{}", e),
        };
        write!(f, "{}\n{}", self.config, info)
    }
}

impl RemoteRepository {
    /// Create a new `RemoteRepository` which acts as a client
    /// to the backend remote repository.
    /// # Error
    ///   - `Error::Api` if the real client could not constructed
    ///  - ``
    pub fn new(config: RusteaConfiguration) -> Result<Self> {
        let c = GiteaClient::new(
            &config.repo.url,
            Some(&config.repo.api_token),
            None,
            &config.repo.repository,
            &config.repo.owner,
        )
        .map_err(Error::Api)?;
        check_folder(&config.script_folder)?;
        Ok(RemoteRepository { config, api: c })
    }

    /// This function queries the remote repository root and
    /// returns a list of `ContentEntry` with `ContentType::Dir`.
    /// All directories in the root are considered as feature sets.
    fn get_feature_sets(&self) -> Result<ContentsResponse> {
        self.api
            .get_file_or_folder("", Some(ContentType::Dir))
            .map_err(Error::Api)
    }

    /// This function returns true if a certain folder in the remote repository root is found.
    fn check_feature_set_exists(&self, name: &str) -> Result<bool> {
        self.get_feature_sets()
            .map(|c| c.content.into_iter().any(|e| e.name == name))
    }

    /// This function prints informations about the remote instance and the
    /// used repository to the command line.
    pub fn info(&self) -> Result<String> {
        Ok(format!(
            "{}\n{}",
            self.api.get_gitea_version()?,
            self.api.get_repository_information()?
        ))
    }

    /// This function prints either the feature sets contained in the remote
    /// repository or if `name` is provided all files found in the feature set.
    pub fn list(&self, feature_set: Option<String>) -> Result<String> {
        let res = match feature_set {
            Some(ref n) => self.api.get_folder(n)?,
            None => self.get_feature_sets()?,
        };
        Ok(format!(
            "{} content:\n{}",
            feature_set.unwrap_or_else(|| String::from(&self.config.repo.repository)),
            res
        ))
    }

    /// This function creates a new feature set within the remote repositories root.
    ///
    /// Since git ignores empty folders, a standard way is used. The file empty
    /// `<featurename>/.gitkeep` is created instead.
    /// If the feature already exists nothing is returned and indicates success,
    /// Normaly the API returns the content entry for the created file but this is
    /// useless in this case. We only check the HTTP return code.
    pub fn new_feature_set(&self, feature_set: &str, cmt_msg: Option<String>) -> Result<String> {
        if !self.check_feature_set_exists(feature_set)? {
            self.api.create_or_update_file(
                feature_set,
                "/.gitkeep",
                "".as_bytes(),
                &self.config.repo.author,
                &self.config.repo.email,
                cmt_msg.as_deref(),
            )?;
            self.api.create_or_update_file(
                feature_set,
                "/scripts/.gitkeep",
                "".as_bytes(),
                &self.config.repo.author,
                &self.config.repo.email,
                cmt_msg.as_deref(),
            )?;
        }
        Ok(format!("Created new feature set {}.", feature_set))
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
        path: Option<String>,
        script: bool,
        recursive: bool,
        cmt_msg: Option<String>,
    ) -> Result<String> {
        let (p, r) = match path {
            Some(path) if script => (format!("{}/scripts/{}", name, path), false),
            Some(path) => (format!("{}/{}", name, path), recursive),
            None => (name.to_owned(), true),
        };
        self.api
            .delete_file_or_folder(
                &p,
                r,
                &self.config.repo.author,
                &self.config.repo.email,
                cmt_msg.as_deref(),
            )
            .map_err(Error::Api)?;
        Ok(format!("Deleted {} successfully.", p))
    }

    /// This function pushes files located in a `path` to the feature set in the remote repository.
    ///
    /// It distinguishes between script files and configuration files through the `script`
    /// argument. The existence of the `path` should be validated beforehand.
    fn push_files(
        &self,
        path: &std::path::Path,
        feature_set: &str,
        script: bool,
        cmt_msg: Option<&str>,
    ) -> Result<()> {
        let files = read_folder(path)?;
        for file in files {
            let remote_path = to_remote_path(&file, script)?;
            let content = read_file(&file)?;
            self.api.create_or_update_file(
                feature_set,
                &remote_path,
                &content,
                &self.config.repo.author,
                &self.config.repo.email,
                cmt_msg,
            )?;
            println!(
                "Pushed file {} into feature set {}",
                remote_path, feature_set
            );
        }
        Ok(())
    }

    /// This function pushes files into a feature set in the remote repository.
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
        path: Option<String>,
        script: bool,
        cmt_msg: Option<String>,
    ) -> Result<String> {
        if !self.check_feature_set_exists(name)? {
            return Err(Error::Rustea(format!("No features set named {}", name)));
        }

        if let Some(path) = path {
            // Push a config or script file or folder
            let path = PathBuf::from(path).canonicalize()?;
            if path.exists() {
                self.push_files(&path, name, script, cmt_msg.as_deref())?;
            } else {
                return Err(Error::Io(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("File {} not found.", path.display()),
                )));
            }
        } else {
            // Push everything found in the feature set
            let feature_set = self.api.get_folder(name)?;
            let script_remote = format!("{}/scripts/", name);

            for entry in feature_set.content {
                let script = entry.path.starts_with(&script_remote);
                let file_path = to_local_path(
                    &entry.path,
                    script,
                    &self.config.script_folder.to_string_lossy(),
                )?;
                if file_path.exists() {
                    self.push_files(&file_path, name, script, cmt_msg.as_deref())?;
                }
            }
        }
        Ok(format!("Files pushed to feature set {}", &name))
    }

    /// This function pulls files from the remote repository.
    ///
    /// It takes a vector of `ContentEntry` converts the path to a local one
    /// depending on the `script` argument. Afterwards, if the path is writable
    /// the files are pulled from the remote repository and gets written to the
    /// local destination. It returns an error if some IO failure happens or
    /// the destination is not writable for the current user.
    fn pull_files(&self, files: &[ContentEntry], script: bool) -> Result<()> {
        for file in files {
            let content = self.api.download_file(&file.path)?;
            let path = to_local_path(
                &file.path,
                script,
                &self.config.script_folder.to_string_lossy(),
            )?;
            // If we have a regular config file, check if the parent folder exists and is writable
            if !script {
                check_folder(&path)?;
            }

            let mut f = File::create(&path)?;
            f.write_all(content.as_bytes()).map_err(Error::Io)?;
            if script {
                let mut perms = f.metadata()?.permissions();
                perms.set_mode(0o751);
                std::fs::set_permissions(&path, perms)?;
            }
            println!("Pulled file {}", path.display());
        }
        Ok(())
    }

    /// This function pulls files from the remote repository and stores them
    /// on the local machine depending on the remote path.
    ///
    /// For the provided feature set either the script files or configuration files
    /// are pulled depending on the `script` and `config` argument. If both are set
    /// to true only script files are pulled to the local machine.
    /// If both arguments are set to false everything if pulled from the feature set.
    ///
    /// ## Attention
    ///
    /// If `path` is provided `script` or `config` flag is set only files matching
    /// the path are pulled. This doesn't distinguishes between remote pathes with the same suffix.
    /// Meaning `/test` and `/example/test` are the same if only `test` is given as path.
    pub fn pull(
        &self,
        name: &str,
        path: Option<String>,
        script: bool,
        config: bool,
    ) -> Result<String> {
        if !self.check_feature_set_exists(name)? {
            return Err(Error::Rustea(format!("No features set named {}", name)));
        }
        let prefix = format!("{}/scripts", name);
        let feature_set = self.api.get_folder(name)?;

        if script || config {
            let files = feature_set
                .content
                .into_iter()
                .filter(|e| {
                    if script {
                        e.path.starts_with(&prefix)
                    } else {
                        // We do not distinguish further between the cases
                        !e.path.starts_with(&prefix)
                    }
                })
                .filter(|e| match &path {
                    Some(p) => e.path.ends_with(p.as_str()),
                    None => true,
                })
                .collect::<Vec<ContentEntry>>();
            self.pull_files(&files, script)?;
        } else {
            // Pull everything found in the feature set
            for file in feature_set.content {
                let script = file.path.starts_with(&prefix);
                self.pull_files(&[file], script)?;
            }
        }
        Ok(format!(
            "Successfully pulled files from feature set {}",
            &name
        ))
    }

    /// This function renames either feature sets or folder and files within the remote repository.
    ///
    /// Provide the feature set `name` in which the files should be moved. If the `path` is
    /// empty the whole feature set is renamed. Otherwise, the `path` is resolved and the
    /// last part of the path (after `/`) is replaced with `new_name`.
    ///
    /// Script files can not be renamed.
    pub fn rename(
        &self,
        name: &str,
        new_name: &str,
        _path: Option<String>,
        cmt_msg: Option<String>,
    ) -> Result<String> {
        if !self.check_feature_set_exists(name)? {
            return Err(Error::Rustea(format!("No features set named {}", name)));
        }
        let feature_set = self.api.get_folder(name)?;

        self.new_feature_set(new_name, None)?;
        for file in feature_set.content {
            let content = self.api.download_file(&file.path)?;
            let base_path = file.path.strip_prefix(name).unwrap();
            self.api.create_or_update_file(
                new_name,
                base_path,
                content.as_bytes(),
                &self.config.repo.author,
                &self.config.repo.email,
                cmt_msg.as_deref(),
            )?;
        }
        self.delete(name, None, false, true, cmt_msg)?;
        Ok(format!(
            "Successfully renamed files in feature set {}",
            name
        ))
    }
}

/// Read a file denoted by a `PathBuf` into a `Vec<u8>` or return the io Error.
fn read_file(path: &std::path::Path) -> Result<Vec<u8>> {
    let mut b: Vec<u8> = Vec::with_capacity(path.metadata()?.len() as usize);
    File::open(path).and_then(|mut f| f.read_to_end(&mut b))?;
    Ok(b)
}

/// This function takes a path and either returns it directly as vector if
/// the path denotes a singles file. Otherwise, the directory is crawled
/// recursively and a vector of all known files below `Path` is returned.
fn read_folder(path: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = vec![];
    let path = path.canonicalize()?;
    if path.is_dir() {
        // Check if the original path is a folder
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            if entry.path().is_dir() {
                // Recursively push folders

                if !entry.path().display().to_string().contains(".git") {
                    let mut entries = read_folder(&entry.path())?;
                    v.append(&mut entries);
                }
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
fn to_remote_path(path: &std::path::Path, script: bool) -> Result<String> {
    match script {
        true => match path.file_name() {
            Some(name) => Ok(format!("/scripts/{}", name.to_string_lossy())),
            None => Err(Error::Io(io::Error::new(
                io::ErrorKind::Other,
                format!("{} not a valid file path", path.display()),
            ))),
        },
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
        None | Some(_) => Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Remote path {} can not converted to local one.",
                remote_path
            ),
        ))),
    }
}

/// This function takes a folder path and creates that path if it
/// not exists and checks if the path is writable afterwards.
fn check_folder(path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        fs::DirBuilder::new().recursive(true).create(&path)?;
    }
    if path.metadata()?.permissions().readonly() {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Path {} not writable.", path.display()),
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
        assert_eq!(remote_path, "/scripts/.gitignore");
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
