use core::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

/// All possible errors which can happen by using the gitea api.
#[derive(Debug)]
pub enum ApiError {
    Reqwest(reqwest::Error),
    Json(serde_json::Error),
    BadApiToken(reqwest::header::InvalidHeaderValue),
    InvalidCredentials(String),
}

impl std::error::Error for ApiError {}

type Result<T> = std::result::Result<T, ApiError>;

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Reqwest(_) => write!(f, "Reqwest"),
            ApiError::Json(_) => write!(f, "Json"),
            ApiError::BadApiToken(_) => write!(f, "Api"),
            ApiError::InvalidCredentials(_) => write!(f, "Credentials"),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Reqwest(err)
    }
}

impl From<reqwest::header::InvalidHeaderValue> for ApiError {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        ApiError::BadApiToken(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Json(err)
    }
}

#[derive(Deserialize, Debug)]
pub struct ApiToken {
    pub id: i64,
    pub name: String,
    pub sha1: String,
    pub token_last_eight: String,
}

/// The gitea version number
#[derive(Deserialize, Debug)]
pub struct Version {
    pub version: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub full_name: String,
    pub created: String,
    pub email: String,
    pub is_admin: bool,
    pub language: String,
    pub last_login: String,
    pub login: String,
    pub restricted: bool,
}

#[derive(Deserialize, Debug)]
pub struct Permission {
    pub admin: bool,
    pub pull: bool,
    pub push: bool,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub empty: bool,
    pub id: i64,
    pub default_branch: String,
    pub description: String,
    pub name: String,
    pub full_name: String,
    pub permissions: Permission,
    pub owner: User,
    pub updated_at: String,
}

#[derive(Deserialize, Debug)]
pub enum ContentType {
    File,
    Dir,
    Symlink,
    Submodule,
}

impl ContentType {
    fn new(stype: &str) -> ContentType {
        match stype {
            "file" => ContentType::File,
            "dir" => ContentType::Dir,
            "symlink" => ContentType::Symlink,
            "submodule" => ContentType::Submodule,
            _ => ContentType::File, //Bad exit
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ContentsResponse {
    pub download_url: String,
    pub name: String,
    pub path: String,
    pub content_type: ContentType,
}

impl ContentsResponse {
    pub fn new(entry: Value) -> ContentsResponse {
        if entry.is_object() {
            ContentsResponse {
                download_url: entry["download_url"].to_string(),
                name: entry["name"].to_string(),
                path: entry["path"].to_string(),
                content_type: ContentType::new(&entry["type"].to_string()),
            }
        } else {
            panic!("Object expected");
        }
    }
}
