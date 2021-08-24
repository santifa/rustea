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
#[macro_use]
extern crate clap;
extern crate base64;
extern crate faccess;
extern crate serde;
extern crate serde_json;
extern crate toml;
extern crate ureq;

use clap::App;
use rustea::Configuration;
use std::process::exit;

// Create the rustea cli
fn app() -> App<'static, 'static> {
    clap_app!(
        rustea =>
            (version: "0.1.2")
            (author: "Henrik Jürges <juerges.henrik@gmail.com")
            (about: "A small utility for fetching configurations from gitea.")
            (@arg CONFIG: -c --config +takes_value "Set a custom configuration file")
             (@arg PRINT: -p --print "Print current configuration")
            (@subcommand init =>
             (about: "Create a new configuration for rustea.")
             (@arg URL: +required "The base url to the gitea instance without the trailing slash.")
             (@arg REPOSITORY: +required "The repository name")
             (@arg OWNER: +required "The repository owner")
             (@arg API_TOKEN: --token +takes_value "Provide the api token for gitea")
             (@arg TOKEN_NAME: --name +takes_value "Provide a name for the api token")
            )
            (@subcommand info =>
             (about: "Show informations about and gitea and the configuration repository."))
            (@subcommand list =>
             (about: "Show the feature sets stored in the repository.")
            (@arg FEATURE: "List the content of a feature set."))
            (@subcommand new =>
             (about: "Create a new empty feature set in the devops repository.")
             (@arg NAME: +required "Name of the feature set"))
            (@subcommand delete =>
             (about: "Delete a feature set or parts of it")
             (@arg RECURSIVE: -r --recursive "Delete a remote folder recursively")
             (@arg SCRIPT: -s --script "Delete a script file from a feature set")
             (@arg NAME: +required "Name of the feature set")
             (@arg PATH: "Path to the configuration files"))
            (@subcommand pull =>
             (about: "Deploy a feature set from the devops repository.")
             (@arg SCRIPT: -s --script "Deploy only the script files of a feature set")
             (@arg CONFIG: -c --config "Deploy only the configuration files of a feature set")
             (@arg NAME: +required "Name of the feature set to pull"))
            (@subcommand push =>
             (about: "Push a feature set to the devops repository.")
             (@arg SCRIPT: -s --script "Push a script file or folder to the devops repository.")
             (@arg NAME: +required "Name of the feature set to push")
             (@arg PATH: "Path to the config or script file or folder"))
    )
}

fn main() {
    let matches = app().get_matches();

    // Print either the default configuration or from the file provided.
    // This is just for convience.
    if matches.is_present("PRINT") {
        let conf = Configuration::read_config_file(matches.value_of("CONFIG"));
        match conf {
            Ok(c) => {
                println!("{}", c);
                exit(0)
            }
            Err(e) => {
                eprintln!("Configuration file not found. Run rustea init --token rustea-devops <repository name> <owner>\nError: {}", e);
                exit(1)
            }
        }
    }

    // We shall evaluate this subcommand before loading the configuration file
    if let Some(sub) = matches.subcommand_matches("init") {
        match Configuration::create_initial_configuration(
            &sub.value_of("URL").unwrap(),
            sub.value_of("API_TOKEN"),
            sub.value_of("TOKEN_NAME"),
            &sub.value_of("REPOSITORY").unwrap(),
            &sub.value_of("OWNER").unwrap(),
        ) {
            Ok(p) => {
                println!(
                    "Successfully initialized rustea. Configuration path {}",
                    p.display()
                );
                exit(0)
            }
            Err(e) => {
                eprintln!("Failed to initialize rustea.\nCause: {}", e);
                exit(1)
            }
        }
    }

    // Now we can safely load the configuration file
    let conf = match Configuration::read_config_file(matches.value_of("CONFIG")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing configuration file: {}", e);
            exit(1)
        }
    };

    // Check which subcommand was used
    match matches.subcommand_name() {
        Some("info") => match conf.repo.info() {
            Ok(_) => exit(0),
            Err(e) => println!("Can not fetch informations. Cause: {}", e),
        },
        Some("list") => {
            let sub = matches.subcommand_matches("list").unwrap();
            match conf.repo.list(sub.value_of("FEATURE")) {
                Ok(_) => exit(0),
                Err(e) => println!("Can not fetch informations. Cause: {}", e),
            }
        }
        Some("new") => {
            let sub = matches.subcommand_matches("new").unwrap();
            match conf.repo.new_feature_set(sub.value_of("NAME").unwrap()) {
                Ok(_) => exit(0),
                Err(e) => println!("Can not fetch informations. Cause: {}", e),
            }
        }
        Some("delete") => {
            let sub = matches.subcommand_matches("delete").unwrap();
            match conf.repo.delete(
                sub.value_of("NAME").unwrap(),
                sub.value_of("PATH"),
                sub.is_present("SCRIPT"),
                sub.is_present("RECURSIVE"),
            ) {
                Ok(_) => println!(
                    "Successfully deleted {} from the feature set {}",
                    sub.value_of("PATH").unwrap(),
                    sub.value_of("NAME").unwrap()
                ),
                Err(e) => eprintln!(
                    "Failed to delete {} from the feature set {}.\nCause: {}",
                    sub.value_of("PATH").unwrap(),
                    sub.value_of("NAME").unwrap(),
                    e
                ),
            }
        }
        Some("pull") => {
            let sub = matches.subcommand_matches("pull").unwrap();
            match conf.repo.pull(
                sub.value_of("NAME").unwrap(),
                &conf.script_folder,
                sub.is_present("SCRIPT"),
                sub.is_present("CONFIG"),
            ) {
                Ok(_) => println!(
                    "Successully pulled files from feature set {}",
                    sub.value_of("NAME").unwrap()
                ),
                Err(e) => eprintln!(
                    "Failed to pull files from feature set {}. Cause {}",
                    sub.value_of("NAME").unwrap(),
                    e
                ),
            }
        }
        Some("push") => {
            let sub = matches.subcommand_matches("push").unwrap();
            match conf.repo.push(
                sub.value_of("NAME").unwrap(),
                &conf.script_folder,
                sub.value_of("PATH"),
                sub.is_present("SCRIPT"),
            ) {
                Ok(_) => println!(
                    "Successfully pushed files to feature set {}",
                    sub.value_of("NAME").unwrap()
                ),
                Err(e) => eprintln!(
                    "Failed to push files to feature set {}. Cause {}",
                    sub.value_of("NAME").unwrap(),
                    e
                ),
            }
        }
        _ => {
            // We have no valid subcommand, but normaly clap checks this case
            println!("Subcommand not found.\n{}", matches.usage());
            exit(1);
        }
    }
    exit(0);
}
