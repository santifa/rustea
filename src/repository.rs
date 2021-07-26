use std::{
    collections::HashMap,
    io::{Error, Write},
};

use crate::gitea;
use gitea::*;
use reqwest::{blocking::Client, header};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

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
    /// Create a new default client for configuration
    pub fn new(
        url: String,
        api_token: Option<&str>,
        username: String,
        repository: String,
    ) -> GiteaClient {
        let client = match api_token {
            // Use the existing token for creation
            Some(token) => GiteaClient {
                url,
                api_token: token.to_string(),
                repository,
                username,
                client: GiteaClient::create_api_client(token),
            },
            // Create a new api token and client configuration
            None => {
                println!("Requesting a new api token.");
                let token_name = read_from_cli("Token name [default: rustea-devops]");
                let token_name = if token_name.is_empty() {
                    "rustea-devops".to_string()
                } else {
                    token_name
                };
                let username = read_from_cli("Username");
                let password = read_from_cli("Password");

                let client = Client::new();
                let response = match client
                    .post(format!("{}/api/v1/users/{}/tokens", url, username))
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(format!("{{\"name\" : \"{}\"}}", token_name))
                    .basic_auth(username.trim(), Some(password.trim()))
                    .send()
                {
                    Ok(res) => res,
                    Err(err) => panic!("Response from api failed with: {:?}", err),
                };

                if response.status().is_success() {
                    let token = response
                        .json::<gitea::ApiToken>()
                        .expect("Failed to parse api token. Already set?");
                    println!(
                        "Got api token number {} with name {}: {}",
                        token.id, token.name, token.token_last_eight
                    );
                    GiteaClient {
                        url,
                        api_token: token.sha1.clone(),
                        repository,
                        username,
                        client: GiteaClient::create_api_client(&token.sha1),
                    }
                } else {
                    panic!("Request failed with {:#?}", response.status())
                }
            }
        };

        // Retreive and print initial informations about the user and repository
        println!("Testing connection to gitea...");
        let gitea_version = client.get_gitea_version().unwrap();
        // let repository = client.get_repository_information().unwrap();
        println!("Gitea version: {:?}", gitea_version);
        // println!("Repoitory: {}", repository.full_name);
        // println!("Owner: {{\n {:#?} \n}}", repository.owner);
        // println!("Permissions: {{\n {:#?} \n}}", repository.permissions);
        // println!("Repository: {{\n {:#?} \n}}", repository);
        client
    }

    /// Construct a new http client.
    /// Since this is a cli tool the client is blocking
    /// and calls are needed in  order by the using functions.
    fn create_api_client(api_token: &str) -> Client {
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
            .unwrap()
    }

    pub fn get_gitea_version(&self) -> Result<Version, ApiError> {
        // let client = self.get_client();
        let result = self
            .client
            .get(format!("{}{}/version", self.url, API_PART))
            .send()?
            .json()
            .map_err(|cause| ApiError::Reqwest(cause));
        result
    }

    pub fn get_repository_information(&self) -> Result<Repository, ApiError> {
        // let client = self.client;
        let result = self
            .client
            .get(format!(
                "{}{}/repos/{}/{}",
                self.url, API_PART, self.username, self.repository
            ))
            .send()?;
        println!("Result: {}", result.status());
        println!("Request: {}", result.url());
        //println!("Request: {:?}", result.text());
        result.json().map_err(|cause| ApiError::Reqwest(cause))
    }

    /// Return the list of content entries with type directory.
    /// Thus are the listed feature set.
    pub fn get_repository_features(&self) -> Result<Vec<ContentsResponse>, ApiError> {
        // let client = self.get_client();
        let result = self
            .client
            .get(format!(
                "{}{}/repos/{}/{}/contents",
                self.url, API_PART, self.username, self.repository,
            ))
            .send()?;
        let v: Value = serde_json::from_str(&result.text().unwrap()).unwrap();
        match v {
            Value::Array(content) => {
                let mut features = vec![];
                for e in content {
                    features.push(ContentsResponse::new(e));
                }
                Ok(features)
            }
            _ => panic!("Array of content expected"),
        }
    }

    /// A simple check if a certain file exists.
    /// It only checks the return code and not if it's realy a folder.
    pub fn check_feature_set_exists(&self, name: &str) -> bool {
        let result = self
            .client
            .get(format!(
                "{}{}/repos/{}/{}/contents/{}",
                self.url, API_PART, self.username, self.repository, name
            ))
            .send()
            .unwrap();

        result.status().is_success()
    }

    pub fn create_new_feature_set(&self) -> () {}
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

#[cfg(test)]
mod tests {
    use crate::Configuration;

    fn load_dev_conf() -> super::GiteaClient {
        crate::load_config(Some("rustea.toml")).get_api_client()
    }

    #[test]
    fn test_get_repository() {
        let client = load_dev_conf();
        let repository = client.get_repository_information();
        println!("{:#?}", repository);
        assert!(repository.is_ok());
    }

    #[test]
    fn test_get_feature_sets() {
        let client = load_dev_conf();
        let feature_sets = client.get_repository_features();
        println!("{:#?}", feature_sets);
        assert!(feature_sets.is_ok());
        let fs = feature_sets.unwrap();
        assert_eq!(1, fs.len());
    }

    #[test]
    fn test_feature_exists() {
        let client = load_dev_conf();
        assert!(client.check_feature_set_exists("README.md"));
    }

    #[test]
    fn test_feature_not_exists() {
        let client = load_dev_conf();
        assert!(!client.check_feature_set_exists("README.dd"));
    }

    #[test]
    fn test_create_feature_set() {
        assert!(false, "To implement");
    }

    #[test]
    fn test_delete_feature_set() {
        assert!(false, "To implement");
    }

    #[test]
    fn test_push_config() {
        assert!(false, "To implement");
    }

    #[test]
    fn test_pull_config() {
        assert!(false, "To implement");
    }
}
