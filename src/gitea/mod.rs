pub mod gitea_api;

use base64::encode;
use reqwest::{blocking::Client, header};
use serde_json::{json, Value};
use std::io::Write;

use gitea_api::{ApiError, ApiResult, ApiToken, ContentsResponse, Repository, Version};

use self::gitea_api::{ContentEntry, ContentType};

const API_PART: &str = "/api/v1";

#[derive(Debug)]
pub struct GiteaClient {
    pub url: String,
    pub api_token: String,
    pub repository: String,
    pub username: String,
    client: Client,
}

impl GiteaClient {
    /// Construct a new http client.
    /// Since this is a cli tool the client is blocking
    /// and calls to the API are made order.
    fn create_api_client(api_token: &str) -> ApiResult<Client> {
        let mut headers = header::HeaderMap::new();
        let auth = format!("token {}", api_token);
        let auth = auth.as_str();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(auth).unwrap(),
        );
        Client::builder()
            .user_agent("rustea")
            .default_headers(headers)
            .build()
            .map_err(ApiError::Reqwest)
    }

    /// This functions requests a new Gitea API token if no one is provided.
    /// It asks the user for a token name, its username and password which is
    /// used for plain authentication against the Gitea API.
    fn create_new_api_token(url: &str, token_name: Option<&str>) -> ApiResult<ApiToken> {
        println!("Requesting a new api token.");
        let username = read_from_cli("Username");
        let password = read_from_cli("Password");

        Client::new()
            .post(format!("{}/api/v1/users/{}/tokens", url, username))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(format!(
                "{{\"name\" : \"{}\"}}",
                token_name.unwrap_or("rustea-devops")
            ))
            .basic_auth(username.trim(), Some(password.trim()))
            .send()?
            .json::<ApiToken>()
            .map_err(ApiError::Reqwest)
    }

    /// This creates a new default Gite API client
    /// which can be used to communicate with some Gitea instance.
    /// It returns an `ApiError` if either the `Reqwest::blocking::client` creation
    /// fails or the creation of a new configuration file.
    pub fn new(
        url: &str,
        api_token: Option<&str>,
        token_name: Option<&str>,
        username: &str,
        repository: &str,
    ) -> ApiResult<GiteaClient> {
        match api_token {
            // Use the existing token for creation
            Some(token) => Ok(GiteaClient {
                url: url.into(),
                api_token: token.to_string(),
                repository: repository.into(),
                username: username.into(),
                client: GiteaClient::create_api_client(token)?,
            }),
            // Create a new api token and client configuration
            None => {
                let token = GiteaClient::create_new_api_token(&url, token_name)?;
                println!("{}", token);

                let client = GiteaClient {
                    url: url.into(),
                    api_token: token.sha1.clone(),
                    repository: repository.into(),
                    username: username.into(),
                    client: GiteaClient::create_api_client(&token.sha1)?,
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
        self.client
            .get(format!("{}{}/version", self.url, API_PART))
            .send()?
            .error_for_status()?
            .json()
            .map_err(ApiError::Reqwest)
    }

    /// Returns informations about the remote repository used by rustea.
    pub fn get_repository_information(&self) -> ApiResult<Repository> {
        self.client
            .get(format!(
                "{}{}/repos/{}/{}",
                self.url, API_PART, self.username, self.repository
            ))
            .send()?
            .error_for_status()?
            .json()
            .map_err(ApiError::Reqwest)
    }

    /// Returns a `Vec` of `ContentEntry` which represents either a folder or
    /// file.
    pub fn get_file_or_folder(
        &self,
        name: &str,
        filter_type: Option<ContentType>,
    ) -> ApiResult<ContentsResponse> {
        let res = self
            .client
            .get(format!(
                "{}{}/repos/{}/{}/contents/{}",
                self.url, API_PART, self.username, self.repository, name
            ))
            .send()?
            .error_for_status()?
            .json::<Value>()?;
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
        content: &str,
        author: &str,
        mail: &str,
    ) -> ApiResult<()> {
        self.client
            .post(format!(
                "{}{}/repos/{}/{}/contents/{}{}",
                self.url, API_PART, self.username, self.repository, feature_name, filename
            ))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(
                json!({"author": { "email": mail, "name": author}, "content": encode(content) })
                    .to_string(),
            )
            .send()?
            .error_for_status()?;
        Ok(())
    }

    /// This function checks wether a file exists under the feature set and either uploads
    /// this file if none-existent or updates the content otherwise.
    pub fn create_or_update_file(
        &self,
        feature_name: &str,
        filename: &str,
        content: &str,
        author: &str,
        mail: &str,
    ) -> ApiResult<()> {
        if self.check_file_exists(feature_name, filename) {
            let files = self.get_file_or_folder(&format!("{}{}", feature_name, filename), None)?;
            let file_sha = files.content[0].sha.as_ref().unwrap();

            self.client
                .put(format!(
                    "{}{}/repos/{}/{}/contents/{}{}",
                    self.url, API_PART, self.username, self.repository, feature_name, filename
                ))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(
                    json!({"author": { "email": mail, "name": author},
                       "content": encode(content), "sha": file_sha })
                    .to_string(),
                )
                .send()?
                .error_for_status()?;
            Ok(())
        } else {
            self.create_file(feature_name, filename, content, author, mail)
        }
    }

    /// This function deletes a file from the remote repository.
    pub fn delete_file(
        &self,
        name: &str,
        file_sha: &str,
        author: &str,
        mail: &str,
    ) -> ApiResult<()> {
        self.client
            .delete(format!(
                "{}{}/repos/{}/{}/contents/{}",
                self.url, API_PART, self.username, self.repository, name
            ))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(json!({"author": { "email": mail, "name": author}, "sha": file_sha }).to_string())
            .send()?
            .error_for_status()?;
        Ok(())
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
    ) -> ApiResult<()> {
        let content = self.get_file_or_folder(name, None)?;

        for file in content.content {
            match file.content_type {
                ContentType::Dir => {
                    if recursive {
                        self.delete_file_or_folder(&file.path, true, author, mail)?
                    } else {
                        self.delete_file(&file.path, file.sha.as_ref().unwrap(), author, mail)?
                    }
                }
                _ => self.delete_file(&file.path, file.sha.as_ref().unwrap(), author, mail)?,
            }
        }
        Ok(())
    }

    // pub fn download(&self, url: &str) -> ApiResult<String> {
    //     Ok(self.client.get(url).send()?.error_for_status()?.text()?)
    // }

    pub fn download_file(&self, name: &str) -> ApiResult<String> {
        let content = self.get_file(name)?;
        Ok(self
            .client
            .get(format!(
                "{}{}/repos/{}/{}/raw/{}",
                self.url, API_PART, self.username, self.repository, content.path
            ))
            .send()?
            .error_for_status()?
            .text()?)
    }

    // This function queries the remote repository root and
    // returns a list of `ContentEntry` with `ContentType::Dir`.
    // All directories in the root are considered as feature sets.
    // pub fn get_repository_features(&self) -> ApiResult<ContentsResponse> {
    //     self.get_file_or_folder("", Some(ContentType::Dir))
    // }

    // /// This function returns true if a certain folder in the remote repository
    // /// root is found.
    // pub fn check_feature_set_exists(&self, name: &str) -> ApiResult<bool> {
    //     let content = self.get_repository_features()?.content;
    //     for e in content {
    //         if e.name == name {
    //             return Ok(true);
    //         }
    //     }
    //     Ok(false)
    // }

    // This function creates a new feature set within the remote repositories root.
    // Since git ignores empty folders, a standard way is used. The file empty
    // `<featurename>/.gitkeep` is created instead.
    // If the feature already exists nothing is returned and indicates success,
    // Normaly the API returns the content entry for the created file but this is
    // useless in this case. We only check the HTTP return code.
    // pub fn create_new_feature_set(&self, feature_name: &str) -> ApiResult<()> {
    //     if self.check_feature_set_exists(feature_name)? {
    //         Ok(())
    //     } else {
    //         self.create_file(feature_name, ".gitkeep", "")?;
    //         self.create_file(feature_name, "scripts/.gitkeep", "")
    //     }
    // }

    // pub fn delete_feature_set(&self, name: &str) -> ApiResult<()> {
    //     self.delete_file_or_folder(name, true)
    // }

    // pub fn delete_conf_from_feature_set(
    //     &self,
    //     name: &str,
    //     path: &str,
    //     recursive: bool,
    // ) -> ApiResult<()> {
    //     self.delete_file_or_folder(&format!("{}/{}", name, path), recursive)
    // }

    // pub fn delete_script_from_feature_set(&self, name: &str, script_name: &str) -> ApiResult<()> {
    //     self.delete_file_or_folder(&format!("{}/scripts/{}", name, script_name), false)
    // }
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
