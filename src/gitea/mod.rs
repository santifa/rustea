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
pub mod gitea_api;

use base64::encode;
use std::io::Write;
use ureq::{Agent, AgentBuilder};

use gitea_api::{ApiError, ApiResult, ApiToken, ContentsResponse, Repository, Version};

use self::gitea_api::{ContentEntry, ContentType};

const API_PART: &str = "/api/v1";

#[derive(Debug)]
pub struct GiteaClient {
    pub url: String,
    pub api_token: String,
    pub repository: String,
    pub owner: String,
    client: Agent,
}

impl Default for GiteaClient {
    fn default() -> Self {
        GiteaClient {
            url: String::with_capacity(0),
            api_token: String::with_capacity(0),
            repository: String::with_capacity(0),
            owner: String::with_capacity(0),
            client: ureq::agent(),
        }
    }
}

impl GiteaClient {
    /// Construct a new http client.
    /// Since this is a cli tool the client is blocking
    /// and calls to the API are made order.
    fn create_api_client(_api_token: &str) -> Agent {
        AgentBuilder::new().user_agent("rustea").build()
    }

    /// This functions requests a new Gitea API token if no one is provided.
    /// It asks the user for a token name, its username and password which is
    /// used for plain authentication against the Gitea API.
    fn create_new_api_token(url: &str, token_name: Option<&str>) -> ApiResult<ApiToken> {
        println!("Requesting a new api token.");
        let username = read_from_cli("Username");
        let password = read_from_cli("Password");
        let auth = base64::encode(format!("{}:{}", username, password).as_bytes());

        let agent = AgentBuilder::new().user_agent("rustea").build();
        agent
            .post(&format!("{}/api/v1/users/{}/tokens", url, username))
            .set("Authorization", &format!("Basic {}", auth))
            .set("content-type", "application/json")
            .send_json(ureq::json!({"name": token_name.unwrap_or("rustea-devops")}))?
            .into_json::<ApiToken>()
            .map_err(ApiError::Io)
    }

    /// This creates a new default Gite API client
    /// which can be used to communicate with some Gitea instance.
    /// It returns an `ApiError` if either the `Reqwest::blocking::client` creation
    /// fails or the creation of a new configuration file.
    pub fn new(
        url: &str,
        api_token: Option<&str>,
        token_name: Option<&str>,
        repository: &str,
        owner: &str,
    ) -> ApiResult<GiteaClient> {
        match api_token {
            // Use the existing token for creation
            Some(token) => Ok(GiteaClient {
                url: url.into(),
                api_token: token.to_string(),
                repository: repository.into(),
                owner: owner.into(),
                client: GiteaClient::create_api_client(token),
            }),
            // Create a new api token and client configuration
            None => {
                println!(
                    "Requesting new topen with name {}",
                    token_name.unwrap_or("rustea-devops")
                );
                let token = GiteaClient::create_new_api_token(&url, token_name)?;
                println!("{}", token);

                let client = GiteaClient {
                    url: url.into(),
                    api_token: token.sha1.clone(),
                    repository: repository.into(),
                    owner: owner.into(),
                    client: GiteaClient::create_api_client(&token.sha1),
                };
                println!("Testing connection to gitea...");
                let gitea_version = client.get_gitea_version()?;
                let repository = client.get_repository_information()?;
                println!("{}\n{}", gitea_version, repository);
                Ok(client)
            }
        }
    }

    /// Returns the Gitea version of the remote instance used by rustea.
    pub fn get_gitea_version(&self) -> ApiResult<Version> {
        // todo!()
        self.client
            .get(&format!("{}{}/version", self.url, API_PART))
            .set("Authorization", &format!("token {}", self.api_token))
            .call()?
            .into_json()
            .map_err(ApiError::Io)
    }

    /// Returns informations about the remote repository used by rustea.
    pub fn get_repository_information(&self) -> ApiResult<Repository> {
        self.client
            .get(&format!(
                "{}{}/repos/{}/{}",
                self.url, API_PART, self.owner, self.repository
            ))
            .set("Authorization", &format!("token {}", self.api_token))
            .call()?
            .into_json()
            .map_err(ApiError::Io)
    }

    /// Returns a `Vec` of `ContentEntry` which represents either a folder or file.
    pub fn get_file_or_folder(
        &self,
        name: &str,
        filter_type: Option<ContentType>,
    ) -> ApiResult<ContentsResponse> {
        let res = self
            .client
            .get(&format!(
                "{}{}/repos/{}/{}/contents/{}",
                self.url, API_PART, self.owner, self.repository, name
            ))
            .set("Authorization", &format!("token {}", self.api_token))
            .call()?
            .into_json()
            .map_err(ApiError::Io)?;
        Ok(ContentsResponse::new(res, filter_type)?)
    }

    /// Utilizes the `get_file_or_folder` function and returns the first found file
    /// as `ContentEntry` if somethin is found.
    /// There is no additional check if the first found file is really the file in question.
    /// Don't use this for folders.
    pub fn get_file(&self, name: &str) -> ApiResult<ContentEntry> {
        let mut res = self.get_file_or_folder(name, Some(ContentType::File))?;
        res.content
            .pop()
            .ok_or(ApiError::InvalidContentResponse(format!(
                "No valid response for the request of file {}",
                name
            )))
    }

    pub fn get_folder(&self, name: &str) -> ApiResult<ContentsResponse> {
        let feature_set = self.get_file_or_folder(name, None)?;
        let mut files = vec![];

        for entity in feature_set.content {
            match entity.content_type {
                ContentType::Dir => {
                    files.append(&mut self.get_folder(&entity.path)?.content);
                }
                _ => {
                    if entity.name != ".gitkeep" {
                        files.push(entity)
                    }
                }
            }
        }
        Ok(ContentsResponse { content: files })
    }

    /// A file exists if the first element of the `ContentsResponse` has the same name
    /// as the requested file.
    pub fn check_file_exists(&self, feature_name: &str, filename: &str) -> bool {
        let content = self.get_file(&format!("{}{}", feature_name, filename));
        match content {
            Ok(c) => c.path == format!("{}{}", feature_name, filename),
            Err(_) => false,
        }
    }

    /// This functions creates a new file within a feature set.
    /// The rustea API distinguishes between file creation and content update.
    pub fn create_file(
        &self,
        feature_name: &str,
        filename: &str,
        content: &[u8],
        author: &str,
        mail: &str,
        cmt_msg: Option<&str>,
    ) -> ApiResult<String> {
        let mut msg = match cmt_msg {
            Some(s) => ureq::json!({ "message": s }),
            None => ureq::json!({}),
        };
        let mut body =
            ureq::json!({"author": { "email": mail, "name": author}, "content": encode(content) });
        body.as_object_mut()
            .unwrap()
            .append(&mut msg.as_object_mut().unwrap());
        self.client
            .post(&format!(
                "{}{}/repos/{}/{}/contents/{}{}",
                self.url, API_PART, self.owner, self.repository, feature_name, filename
            ))
            .set("Authorization", &format!("token {}", self.api_token))
            .set("content-type", "application/json")
            .send_json(body)?
            .into_string()
            .map_err(ApiError::Io)
    }

    /// This function checks wether a file exists under the feature set and either uploads
    /// this file if none-existent or updates the content otherwise.
    pub fn create_or_update_file(
        &self,
        feature_name: &str,
        filename: &str,
        content: &[u8],
        author: &str,
        mail: &str,
        cmt_msg: Option<&str>,
    ) -> ApiResult<String> {
        if self.check_file_exists(feature_name, filename) {
            let files = self.get_file_or_folder(&format!("{}{}", feature_name, filename), None)?;
            let file_sha = files.content[0].sha.as_ref().unwrap();

            let mut msg = match cmt_msg {
                Some(s) => ureq::json!({ "message": s }),
                None => ureq::json!({}),
            };
            let mut body = ureq::json!({"author": { "email": mail, "name": author}, "content": encode(content), "sha": file_sha, "message": cmt_msg });

            body.as_object_mut()
                .unwrap()
                .append(&mut msg.as_object_mut().unwrap());

            self.client
                .put(&format!(
                    "{}{}/repos/{}/{}/contents/{}{}",
                    self.url, API_PART, self.owner, self.repository, feature_name, filename
                ))
                .set("Authorization", &format!("token {}", self.api_token))
                .set("content-type", "application/json")
                .send_json(body)?
                .into_string()
                .map_err(ApiError::Io)
        } else {
            self.create_file(feature_name, filename, content, author, mail, cmt_msg)
        }
    }

    /// This function deletes a file from the remote repository.
    pub fn delete_file(
        &self,
        name: &str,
        file_sha: &str,
        author: &str,
        mail: &str,
        cmt_msg: Option<&str>,
    ) -> ApiResult<String> {
        let mut msg = match cmt_msg {
            Some(s) => ureq::json!({ "message": s }),
            None => ureq::json!({}),
        };
        let mut body = ureq::json!({"author": { "email": mail, "name": author}, "sha": file_sha , "message": cmt_msg });

        body.as_object_mut()
            .unwrap()
            .append(&mut msg.as_object_mut().unwrap());

        self.client
            .delete(&format!(
                "{}{}/repos/{}/{}/contents/{}",
                self.url, API_PART, self.owner, self.repository, name
            ))
            .set("Authorization", &format!("token {}", self.api_token))
            .send_json(ureq::json!({"author": { "email": mail, "name": author}, "sha": file_sha , "message": cmt_msg }))?
            .into_string()
            .map_err(ApiError::Io)
    }

    /// This functions deletes either a file or the whole folder from
    /// the remote repository.
    /// The function can recursively delete folders
    pub fn delete_file_or_folder(
        &self,
        name: &str,
        recursive: bool,
        author: &str,
        mail: &str,
        cmt_msg: Option<&str>,
    ) -> ApiResult<()> {
        let content = self.get_file_or_folder(name, None)?;

        for file in content.content {
            match file.content_type {
                ContentType::Dir => {
                    if recursive {
                        self.delete_file_or_folder(&file.path, true, author, mail, cmt_msg)?;
                    } else {
                        self.delete_file(
                            &file.path,
                            file.sha.as_ref().unwrap(),
                            author,
                            mail,
                            cmt_msg,
                        )?;
                    }
                }
                _ => {
                    self.delete_file(
                        &file.path,
                        file.sha.as_ref().unwrap(),
                        author,
                        mail,
                        cmt_msg,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn download_file(&self, name: &str) -> ApiResult<String> {
        let content = self.get_file(name)?;
        self.client
            .get(&format!(
                "{}{}/repos/{}/{}/raw/{}",
                self.url, API_PART, self.owner, self.repository, content.path
            ))
            .set("Authorization", &format!("token {}", self.api_token))
            .call()?
            .into_string()
            .map_err(ApiError::Io)
    }
}

/// Read user input from the commandline.
/// Provide a short description about what to enter.
/// Returns None if the user enters an empty line.
fn read_from_cli(prefix: &str) -> String {
    print!("{}: ", prefix);
    std::io::stdout()
        .flush()
        .expect("Error flushing to stdout.");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_owned()
}
