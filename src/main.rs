#[macro_use]
extern crate clap;
extern crate reqwest;
#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate toml;

mod gitea;
mod repository;

use clap::ArgMatches;
use core::fmt;
use repository::GiteaClient;
use serde_derive::{Deserialize, Serialize};
use std::{
    env,
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::Path,
    process::exit,
};
//use serde_derive::Deserialize;

// Assume that the configuration file is either in
// in the users home directory or provided on the cli.
const DEFAULT_CONF: &str = ".rustea.toml";

fn get_default_path() -> String {
    match env::var_os("HOME") {
        Some(val) => {
            let home = String::from(val.to_str().unwrap());
            home + "/" + DEFAULT_CONF
        }
        None => panic!("Could not find home"),
    }
}

#[derive(Deserialize, Serialize)]
struct RepositoryConfig {
    url: String,
    api_token: String,
    repository: String,
    username: String,
}

impl Display for RepositoryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\turl={}\n\tapi_token={}\n\trepository={}\n\tusername={}\n",
            self.url, self.api_token, self.repository, self.username
        )
    }
}

#[derive(Deserialize, Serialize)]
struct Configuration {
    script_folder: String,
    client: RepositoryConfig,
}

impl Configuration {
    /**
     * Parse and validate cli arguments and configuration.
     */
    fn parse_file(file_path: &str) -> Result<Configuration, std::io::Error> {
        let mut config_string = String::new();
        match File::open(file_path) {
            Ok(mut file) => {
                file.read_to_string(&mut config_string).unwrap();
            }

            Err(error) => {
                return Err(error);
            }
        }
        Ok(toml::from_str(&config_string).unwrap())
    }

    fn write_file(&self, file_path: &str) -> Result<(), std::io::Error> {
        let conf_string = toml::to_string_pretty(self).unwrap();
        let mut file = match File::create(file_path) {
            Err(cause) => panic!("Could not create {} : {}", file_path, cause),
            Ok(file) => file,
        };
        file.write_all(conf_string.as_bytes())
    }

    fn get_api_client(&self) -> GiteaClient {
        GiteaClient::new(
            self.client.url.clone(),
            Some(&self.client.api_token),
            self.client.username.clone(),
            self.client.repository.clone(),
        )
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "script_folder={}\nrepo={{\n{}}}",
            self.script_folder, self.client
        )
    }
}

// Load the configuration file and prefer the argument value over default value
fn load_config(arg: Option<&str>) -> Configuration {
    let default = get_default_path();
    let path = match arg {
        Some(p) => p,
        None => &default,
    };
    Configuration::parse_file(path)
        .unwrap_or_else(|error| panic!("Error parsing configuration file: {}, {}", path, error))
}

/// This function creates a new configuration file called .rustea.toml
/// in the callers home directory. It either creates a new api key
/// or uses a provided one for connecting to the api.
fn initialize_rustea(matches: &ArgMatches) {
    let client = GiteaClient::new(
        matches.value_of("URL").unwrap().to_string(),
        matches.value_of("API_TOKEN"),
        matches.value_of("REPOSITORY").unwrap().to_string(),
        matches.value_of("OWNER").unwrap().to_string(),
    );
    let conf = Configuration {
        script_folder: "/usr/local/bin".to_owned(),
        client: RepositoryConfig {
            url: client.url,
            api_token: client.api_token,
            repository: client.repository,
            username: client.username,
        },
    };

    let path = get_default_path();
    println!("Path {:?}", path);

    match conf.write_file(&path) {
        Ok(_) => println!("Configuration successfully written to {}", path),
        Err(cause) => panic!(
            "Could not write configuration file {} : {}",
            DEFAULT_CONF, cause
        ),
    }
}

fn main() {
    let matches = clap_app!(
        rustea =>
            (version: "0.1")
            (author: "Henrik Jürges <juerges.henrik@gmail.com")
            (about: "A small utility for fetching configurations from gitea.")
            (@arg CONFIG: -c --config +takes_value "Set a custom configuration file")
            (@arg GITEA: --info "Print informations about the gitea instance and repository")
             (@arg PRINT: -p --print "Print current configuration")
            (@subcommand init =>
             (about: "Create a rustea configuration")
             (@arg URL: +required "The base url to the gitea instance")
             (@arg REPOSITORY: +required "The repository name")
             (@arg OWNER: +required "The repository owner")
             (@arg API_TOKEN: --token +takes_value "Provide the api token for gitea")
             (@arg TOKEN_NAME: --name +takes_value "Provide a name for the api token")
            )
            (@subcommand info =>
            (about: "Show informations about the configuration repository and gitea"))
            (@subcommand list =>
            (about: "Show the feature sets stored in the repository"))
            (@subcommand new =>
             (about: "Create a new feature set in the devops repository")
             (@arg NAME: +required "Name of the feature set")
            )
            (@subcommand pull =>
             (about: "Deploy a feature set from the devops repository.")
             (@arg NAME: +required "Name of the feature set to pull")
            )
            (@subcommand push =>
             (about: "Push a feature set to the devops repository.")
             (@arg NAME: +required "Name of the feature set to push")
            )
    )
    .get_matches();

    if matches.is_present("PRINT") {
        let default = get_default_path();
        let path = Path::new(matches.value_of("CONFIG").unwrap_or(&default));
        if path.exists() {
            let conf = load_config(matches.value_of("CONFIG"));
            println!("Found configuration {}\n{}", path.display(), conf);
            exit(0);
        } else {
            println!("Configuration file not found. Have you initialized rustea?");
            exit(1);
        }
    }

    // We shall evaluate this subcommand before loading the configuration file
    if let Some(sub) = matches.subcommand_matches("init") {
        // Create a new configuration file and initialize the api and repository
        initialize_rustea(sub);
        exit(0);
    }

    // Now we can safely load the configuration file
    let conf = load_config(matches.value_of("CONFIG"));

    // Check which subcommand was used
    match matches.subcommand_name() {
        Some("info") => {
            let api = conf.get_api_client();
            let gitea_version = api.get_gitea_version().unwrap();
            let repository = api.get_repository_information().unwrap();
            println!("Gitea {:#?}", gitea_version);
            println!("Respository {:#?}", repository);
        }
        Some("list") => {
            let api = conf.get_api_client();
            let feature_sets = api.get_repository_features();
            println!("Feature Sets:\n{:#?}", feature_sets);
        }
        Some("new") => {}
        Some("pull") => {}
        Some("push") => {}
        _ => {
            // We have no valid subcommand, but normaly clap checks this case
            println!("Subcommand not found.");
            exit(1);
        }
    }
}

/**
 * TESTS
 */
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_config() {
        let conf_result = Configuration::parse_file("rustea.toml");
        assert!(conf_result.is_ok());
        let conf = conf_result.unwrap();
        assert_eq!(conf.script_folder, "/usr/local/bin");
    }

    #[test]
    fn test_no_config_file() {
        let conf_result = Configuration::parse_file("");
        assert!(conf_result.is_err());
    }
}
