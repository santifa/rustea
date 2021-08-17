use core::fmt;
use std::{fmt::Display, io};

use serde_derive::Deserialize;
use serde_json::Value;

/// All possible errors which can happen by using the gitea api.
#[derive(Debug)]
pub enum ApiError {
    Reqwest(reqwest::Error),
    Json(serde_json::Error),
    BadApiToken(reqwest::header::InvalidHeaderValue),
    InvalidCredentials(String),
    InvalidContentResponse(String),
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            ApiError::Reqwest(ref c) => Some(c),
            ApiError::Json(ref c) => Some(c),
            ApiError::BadApiToken(ref c) => Some(c),
            ApiError::InvalidCredentials(_) => None,
            ApiError::InvalidContentResponse(_) => None,
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match *self {
            ApiError::Reqwest(ref c) => Some(c),
            ApiError::Json(ref c) => Some(c),
            ApiError::BadApiToken(ref c) => Some(c),
            ApiError::InvalidCredentials(_) => None,
            ApiError::InvalidContentResponse(_) => None,
        }
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Reqwest(_) => write!(f, "Reqwest"),
            ApiError::Json(_) => write!(f, "Json"),
            ApiError::BadApiToken(_) => write!(f, "Api"),
            ApiError::InvalidCredentials(_) => write!(f, "Credentials"),
            ApiError::InvalidContentResponse(_) => write!(f, "Content response"),
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

impl Display for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Api Token number {}, name {}: {}",
            self.id, self.name, self.token_last_eight
        )
    }
}

/// The gitea version number
#[derive(Deserialize, Debug)]
pub struct Version {
    pub version: String,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gitea version: {}", self.version)
    }
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

impl Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Owner {} {{\n\t\tName: {}\n\t\tCreated: {}\n\t\tMail: {}\n\t\tAdmin: {}\n\t\tLang: {}\n\t\tLast Login: {}\n\t\tLogin: {}\n\t\tRestricted: {}\n\t}}",
            self.id,
            self.full_name,
            self.created,
            self.email,
            self.is_admin,
            self.language,
            self.last_login,
            self.login,
            self.restricted,
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct Permission {
    pub admin: bool,
    pub pull: bool,
    pub push: bool,
}

impl Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Permissions: admin[{}], pull[{}], push[{}]",
            self.admin, self.pull, self.push
        )
    }
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

impl Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Repository {} {{\n\tName: {}\n\tFull Name: {}\n\tDescription: {}\n\tEmpty: {}\n\tUpdated At: {}\n\t{}\n\t{}\n}}", self.id, self.name, self.full_name, self.description, self.empty, self.updated_at, self.permissions, self.owner)
    }
}

/// The content type describes which type of "file"
/// is found by gitea for a specific path or listing.
/// If the content type is unknown the implementation returns
/// a file as default type.
#[derive(Debug, PartialEq)]
pub enum ContentType {
    File,
    Dir,
    Symlink,
    Submodule,
}

impl ContentType {
    /// Create a new content type from a string.
    /// Returns `ContentType::File` if no valid content type is found.
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

impl Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentType::File => write!(f, "File"),
            ContentType::Dir => write!(f, "Dir"),
            ContentType::Symlink => write!(f, "Symlink"),
            ContentType::Submodule => write!(f, "Submodule"),
        }
    }
}

#[derive(Debug)]
pub struct ContentEntry {
    pub download_url: Option<String>,
    pub name: String,
    pub path: String,
    pub content_type: ContentType,
    pub sha: Option<String>,
}

impl Display for ContentEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\t\t\t{}\t\t\t\t{}\t\t{}",
            self.name,
            self.path,
            self.content_type,
            self.sha.as_ref().unwrap_or(&"No SHA".to_string())
        )
    }
}

impl ContentEntry {
    pub fn new(entry: Value) -> ApiResult<ContentEntry> {
        if entry.is_object() {
            // println!("{}", entry["content"]);
            Ok(ContentEntry {
                download_url: entry["download_url"].as_str().map(String::from),
                sha: entry["sha"].as_str().map(String::from),
                name: entry["name"]
                    .as_str()
                    .ok_or(ApiError::InvalidContentResponse(
                        "File name missing.".into(),
                    ))?
                    .to_string(),
                path: entry["path"]
                    .as_str()
                    .ok_or(ApiError::InvalidContentResponse(
                        "File path missing.".into(),
                    ))?
                    .to_string(),
                content_type: ContentType::new(entry["type"].as_str().ok_or(
                    ApiError::InvalidContentResponse("Content type missing.".into()),
                )?),
            })
        } else {
            Err(ApiError::InvalidContentResponse(
                "A valid content object is needed.".into(),
            ))
        }
    }
}

/// A handy struct definition for the list of content entries.
#[derive(Debug)]
pub struct ContentsResponse {
    pub content: Vec<ContentEntry>,
}

impl Display for ContentsResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name\t\t\t\tPath\t\t\t\tType\t\tSHA\n")?;
        for entry in &self.content {
            write!(f, "{}\n", entry)?;
        }
        Ok(())
    }
}

impl ContentsResponse {
    /// This function creates a new list of content entries called `ContentsResponse` by
    /// Gitea. It returns an error if the `Value` is not a `Value::Array`, `Value::Object` or
    /// the json objects are not valid content entries.
    /// If only one content object (e.g. a file) is provided it is converted to an array with only
    /// one entry.
    pub fn new(content: Value, type_filter: Option<ContentType>) -> ApiResult<ContentsResponse> {
        match content {
            Value::Array(entries) => {
                let mut c = vec![];
                for e in entries {
                    let entry = ContentEntry::new(e)?;
                    if let Some(ref t) = type_filter {
                        // add only one type of files
                        if entry.content_type == *t {
                            c.push(entry);
                        }
                    } else {
                        // Add all file types
                        c.push(entry);
                    }
                }
                Ok(ContentsResponse { content: c })
            }
            Value::Object(_) => Ok(ContentsResponse {
                content: vec![ContentEntry::new(content)?],
            }),
            _ => Err(ApiError::InvalidContentResponse(
                "Only json arrays are valid content responses".into(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::Value;

    use super::{ContentType, ContentsResponse};

    #[test]
    fn test_content_response_new() {
        let v: Value = serde_json::from_str("[{\"download_url\": \"test_url\", \"name\": \"test_name\", \"path\": \"test_path\", \"type\": \"dir\"}]").unwrap();
        // let content = ContentsResponse::new(v).unwrap();
        let content = ContentsResponse::new(v, None);
        println!("{:#?}", content);
        let content = content.unwrap();
        assert_eq!(
            content.content[0].download_url.as_ref().unwrap(),
            "test_url"
        );
        assert_eq!(content.content[0].name, "test_name");
        assert_eq!(content.content[0].path, "test_path");
        assert_eq!(content.content[0].content_type, ContentType::Dir);
    }

    #[test]
    fn test_content_response_wrong_content_type() {
        let v: Value = serde_json::from_str("[{\"download_url\": \"test_url\", \"name\": \"test_name\", \"path\": \"test_path\", \"type\": \"d\"}]").unwrap();
        let content = ContentsResponse::new(v, None);
        println!("{:#?}", content);
        let content = content.unwrap();
        assert_eq!(
            content.content[0].download_url.as_ref().unwrap(),
            "test_url"
        );
        assert_eq!(content.content[0].name, "test_name");
        assert_eq!(content.content[0].path, "test_path");
        assert_eq!(content.content[0].content_type, ContentType::File);
    }

    #[test]
    fn test_content_response_no_download_url() {
        let v: Value = serde_json::from_str(
            "[{\"name\": \"test_name\", \"path\": \"test_path\", \"type\": \"d\"}]",
        )
        .unwrap();
        let content = ContentsResponse::new(v, None);
        println!("{:#?}", content);
        let content = content.unwrap();
        assert!(content.content[0].download_url.is_none());
        assert_eq!(content.content[0].name, "test_name");
        assert_eq!(content.content[0].path, "test_path");
        assert_eq!(content.content[0].content_type, ContentType::File);
    }

    #[test]
    fn test_content_response_is_object() {
        let v: Value = serde_json::from_str("{\"download_url\": \"test_url\", \"name\": \"test_name\", \"path\": \"test_path\", \"type\": \"d\"}").unwrap();
        let content = ContentsResponse::new(v, None);
        assert!(content.is_ok());
    }

    #[test]
    fn test_new_content_response_empty_array() {
        let v: Value = serde_json::from_str("[]").unwrap();
        let content = ContentsResponse::new(v, None);
        assert!(content.is_ok());
        assert_eq!(0, content.unwrap().content.len());
    }

    #[test]
    fn test_new_content_response_not_an_object() {
        let v: Value = serde_json::from_str("[1,2]").unwrap();
        let content = ContentsResponse::new(v, None);
        assert!(content.is_err());
    }
}
