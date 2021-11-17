//! This file implements a small self updater facility for rustea.
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

/// Static repository configuration for the self-updater
const OWNER: &str = "santifa";
const REPO: &str = "rustea";
const URL: &str = "https://api.github.com";
const MIME_TYPE: &str = "application/vnd.github.v3+json";
const CUR_VERSION: &str = env!("CARGO_PKG_VERSION");

use std::{
    env,
    io::{Read, Write},
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
};

use crate::error::{Error, Result};
use serde_derive::Deserialize;
use ureq::AgentBuilder;

#[derive(Deserialize, Debug)]
struct Release {
    name: String,
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

impl Release {
    // This function returns either the minified or normal binary
    // download url. At the moment the files are hard-coded.
    fn get_download_url(&self, minified: bool) -> String {
        match minified {
            true => self.assets[1].browser_download_url.to_owned(),
            false => self.assets[0].browser_download_url.to_owned(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, PartialEq, PartialOrd)]
struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

impl Version {
    fn new(s: &str) -> Result<Self> {
        let version = if s.starts_with('v') {
            s.strip_prefix('v').unwrap_or("0.0.0")
        } else {
            s
        };
        let parts: Result<Vec<u8>> = version
            .split('.')
            .map(|e| e.parse::<u8>().map_err(Error::Version))
            .collect();
        let parts = parts?;
        Ok(Version {
            major: parts[0],
            minor: parts[1],
            patch: parts[2],
        })
    }
}

pub struct Updater {
    binary_path: PathBuf,
}

impl Updater {
    // Create a new updater which figures out its
    // own binary path and checks the permissions.
    pub fn new() -> Result<Self> {
        let binary_path = std::env::current_exe()?;
        if binary_path.metadata()?.permissions().readonly() {
            return Err(Error::Rustea(format!(
                "Path {} is not writable",
                binary_path.display()
            )));
        }
        Ok(Updater { binary_path })
    }

    // Set the binary as executable. This should be done after the update.
    fn set_executable(&self) -> Result<()> {
        let mut perms = self.binary_path.metadata()?.permissions();
        perms.set_mode(0o751);
        std::fs::set_permissions(&self.binary_path, perms).map_err(Error::Io)
    }

    // This functions takes a binary buffer and replaces the original
    // executable file with this content by moving the old to a *.bak
    // file and write the content as the new binary. If the write fails
    // the old files is moved back to the original path.
    fn replace_binary(&self, content: &[u8]) -> Result<()> {
        let tmp_bin = self
            .binary_path
            .parent()
            .unwrap_or(&PathBuf::from("/"))
            .join("rustea.bak");
        std::fs::rename(&self.binary_path, &tmp_bin)?;

        let mut f = std::fs::File::create(&self.binary_path)?;
        if let Err(e) = f.write_all(content).map_err(Error::Io) {
            std::fs::rename(&tmp_bin, &self.binary_path)?;
            return Err(e);
        }
        self.set_executable()?;
        std::fs::remove_file(tmp_bin).map_err(Error::Io)
    }

    pub fn update(&self, minified: bool) -> Result<String> {
        let agent = AgentBuilder::new().build();
        // get all releases but we only care for the last one
        let release = agent
            .get(&format!("{}/repos/{}/{}/releases", URL, OWNER, REPO))
            .set("Accept", MIME_TYPE)
            .call()?
            .into_json::<Vec<Release>>()?;

        if let Some(release) = release.first() {
            if Version::new(CUR_VERSION)? < Version::new(&release.tag_name)? {
                let url = release.get_download_url(minified);
                let mut reader = agent.get(&url).call()?.into_reader();
                let mut buffer = Vec::new();
                reader.read_to_end(&mut buffer)?;
                self.replace_binary(&buffer)?;
                Ok(format!("Updated to version {}", release.tag_name))
            } else {
                Err(Error::Rustea("Nothing to update".to_string()))
            }
        } else {
            Err(Error::Rustea(
                "Failed to fetch the latest release from github.".to_string(),
            ))
        }
    }
}
