//! This is library part of the rustea implementation.
//!
//! It implements the heavy lifting for the main binary.

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
pub mod gitea;

use core::fmt;
use faccess::PathExt;
use gitea::{
    gitea_api::{self, ContentEntry, ContentsResponse},
    GiteaClient,
};
use serde_derive::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    fmt::Display,
    fs::{self, File},
    io,
    io::{Read, Write},
    path::PathBuf,
};
use tabwriter::TabWriter;

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
pub const VERSION: &str = "0.1.3";

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
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Configuration {
    pub script_folder: String,
    pub repo: RemoteRepository,
}

impl Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = PathBuf::from(&self.script_folder).canonicalize();

        write!(
            f,
            "Using rustea version {}\nscript_folder = {}\nrepo = {{\n{}\n}}",
            VERSION,
            p.unwrap().display(),
            self.repo
        )
    }
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
                owner: client.owner.clone(),
                email: String::new(),
                author: client.owner.clone(),
            },
        };

        let path = PathBuf::from(get_default_path()?);
        conf.write_config_file(&path).and(Ok(path))
    }
}

/// This struct defines the access to the remote repository
/// which contains the features sets used by rustea.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RemoteRepository {
    pub url: String,
    pub api_token: String,
    pub repository: String,
    pub owner: String,
    pub email: String,
    pub author: String,
}

impl Display for RemoteRepository {
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
        tw.flush().unwrap();
        let written = String::from_utf8(tw.into_inner().unwrap()).unwrap();
        write!(f, "{}", written)
    }
}

impl RemoteRepository {
    /// This function constructs a new Gitea API client for requests.
    fn create_api_client(&self) -> Result<GiteaClient> {
        GiteaClient::new(
            &self.url,
            Some(&self.api_token),
            None,
            &self.repository,
            &self.owner,
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
    pub fn new_feature_set(&self, feature_set: &str, cmt_msg: Option<&str>) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, feature_set)? {
            api.create_or_update_file(
                feature_set,
                "/.gitkeep",
                "".as_bytes(),
                &self.author,
                &self.email,
                cmt_msg,
            )?;
            api.create_or_update_file(
                feature_set,
                "/scripts/.gitkeep",
                "".as_bytes(),
                &self.author,
                &self.email,
                cmt_msg,
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
        cmt_msg: Option<&str>,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        match path {
            Some(path) if script => api.delete_file_or_folder(
                &format!("{}/scripts/{}", name, path),
                false,
                &self.author,
                &self.email,
                cmt_msg,
            ),
            Some(path) => api.delete_file_or_folder(
                &format!("{}/{}", name, path),
                recursive,
                &self.author,
                &self.email,
                cmt_msg,
            ),
            None => api.delete_file_or_folder(name, true, &self.author, &self.email, cmt_msg),
        }
        .map_err(Error::ApiError)
    }

    /// This function pushes files located in a `path` to the feature set in the remote repository.
    ///
    /// It distinguishes between script files and configuration files through the `script`
    /// argument. The existence of the `path` should be validated beforehand.
    fn push_files(
        &self,
        api: &GiteaClient,
        path: &PathBuf,
        // files: &[PathBuf],
        feature_set: &str,
        script: bool,
        cmt_msg: Option<&str>,
    ) -> Result<()> {
        let files = read_folder(&path)?;
        for file in files {
            let remote_path = to_remote_path(&file, script)?;
            let content = read_file(&file)?;
            api.create_or_update_file(
                feature_set,
                &remote_path,
                &content,
                &self.author,
                &self.email,
                cmt_msg,
            )?;
            println!(
                "Pushed file {} into feature set {}",
                remote_path, feature_set
            );
        }
        Ok(())
    }

    /// This function pushes files a feature set in the remote repository.
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
        cmt_msg: Option<&str>,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, name)? {
            return Err(Error::Push(format!("No features set named {}", name)));
        }

        if let Some(path) = path {
            // Push a config or script file or folder
            let path = PathBuf::from(path).canonicalize()?;
            if path.exists() {
                self.push_files(&api, &path, name, script, cmt_msg)?;
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
                let script = entry.path.starts_with(&script_remote);
                let file_path = to_local_path(&entry.path, script, script_dir)?;
                if file_path.exists() {
                    self.push_files(&api, &file_path, name, script, cmt_msg)?;
                }
            }
        }
        Ok(())
    }

    /// This function pulls files from the remote repository.
    ///
    /// It takes a vector of `ContentEntry` converts the path to a local one
    /// depending on the `script` argument. Afterwards, if the path is writable
    /// the files are pulled from the remote repository and gets written to the
    /// local destination. It returns an error if some IO failure happens or
    /// the destination is not writable for the current user.
    fn pull_files(
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
            if script {
                let mut perms = f.metadata()?.permissions();
                perms.set_mode(0o751);
                f.set_permissions(perms)?;
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
        path: Option<&str>,
        script_dir: &str,
        script: bool,
        config: bool,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, name)? {
            return Err(Error::Push(format!("No features set named {}", name)));
        }
        check_folder(script_dir)?;
        let prefix = format!("{}/scripts", name);
        let feature_set = api.get_folder(name)?;

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
                .filter(|e| match path {
                    Some(p) => e.path.ends_with(&p),
                    None => true,
                })
                .collect::<Vec<ContentEntry>>();
            self.pull_files(&api, &files, script, script_dir)?;
        } else {
            // Pull everything found in the feature set
            for file in feature_set.content {
                let script = file.path.starts_with(&prefix);
                self.pull_files(&api, &[file], script, script_dir)?;
            }
        }
        Ok(())
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
        _path: Option<&str>,
        cmt_msg: Option<&str>,
    ) -> Result<()> {
        let api = self.create_api_client()?;
        if !self.check_feature_set_exists(&api, name)? {
            return Err(Error::Push(format!("No features set named {}", name)));
        }
        let feature_set = api.get_folder(name)?;

        self.new_feature_set(new_name, None)?;
        for file in feature_set.content {
            let content = api.download_file(&file.path)?;
            let base_path = file.path.strip_prefix(name).unwrap();
            api.create_or_update_file(
                new_name,
                &base_path,
                content.as_bytes(),
                &self.author,
                &self.email,
                cmt_msg,
            )?;
        }
        self.delete(name, None, false, true, cmt_msg)?;

        // match path {
        //     Some(p) => {
        //         // Distinguish between script files and normal files by existence
        //         let files = feature_set.content.into_iter().find(|e| e.path.ends_with(p));

        //         // Fetch the folder or file
        //         let files = api.get_folder(pbuf?.to_str().unwrap())?;
        //         // Convert old path to new one
        //         let new_path = match p.split_once("/") {
        //             Some((_, path)) => format!("{}/{}", path, new_name),
        //             None => new_name.into(),
        //         };

        //         for file in files.content {
        //             let content = api.download_file(&file.path)?;

        //             //let base_path = file.path;

        //             api.create_or_update_file(
        //                 name,
        //                 &new_path,
        //                 content.as_bytes(),
        //                 &self.author,
        //                 &self.email,
        //                 cmt_msg,
        //             )?;
        //         }
        //         self.delete(name, path, false, true, cmt_msg)?;
        //     }
        //     None => {
        //         // Rename the feature set
        //         self.new_feature_set(new_name, None)?;
        //         for file in feature_set.content {
        //             let content = api.download_file(&file.path)?;
        //             let base_path = file.path.strip_prefix(name).unwrap();
        //             api.create_or_update_file(
        //                 new_name,
        //                 &base_path,
        //                 content.as_bytes(),
        //                 &self.author,
        //                 &self.email,
        //                 cmt_msg,
        //             )?;
        //         }
        //         self.delete(name, None, false, true, cmt_msg)?;
        //     }
        // }
        Ok(())
    }
}

/// Read a file denoted by a `PathBuf` into a `Vec<u8>` or return the io Error.
fn read_file(path: &PathBuf) -> Result<Vec<u8>> {
    let mut b: Vec<u8> = Vec::with_capacity(path.metadata()?.len() as usize);
    File::open(path.as_path()).and_then(|mut f| f.read_to_end(&mut b))?;
    Ok(b)
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
