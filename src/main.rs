extern crate argh;
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
extern crate base64;
extern crate faccess;
extern crate self_update;
extern crate serde;
extern crate serde_json;
extern crate tabwriter;
extern crate toml;
extern crate ureq;

use argh::FromArgs;
use rustea::RusteaConfiguration;
use self_update::cargo_crate_version;
use std::process::exit;

#[derive(FromArgs, PartialEq, Debug)]
/// A simple gitea based configuration management.
struct Rustea {
    /// provide a custom configuration file
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// a commit message used for changing the remote repository
    #[argh(option, short = 'm')]
    message: Option<String>,

    #[argh(subcommand)]
    cmd: RusteaCmd,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum RusteaCmd {
    Init(RusteaInit),
    Info(RusteaInfo),
    List(RusteaList),
    New(RusteaNew),
    Delete(RusteaDelete),
    Pull(RusteaPull),
    Push(RusteaPush),
    Rename(RusteaRename),
    Update(RusteaUpdate),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "update")]
/// Run the rustea self-updater.
struct RusteaUpdate {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "init")]
/// Create a new configuration for rustea.
struct RusteaInit {
    /// provide an api token for the remote repository
    #[argh(option, short = 't')]
    api_token: Option<String>,

    /// provide a name for the api token
    #[argh(option, short = 'n')]
    token_name: Option<String>,

    /// the base url for the gitea instance without trailing slash
    #[argh(positional)]
    url: String,

    /// the name of the remote repository
    #[argh(positional)]
    repository: String,

    /// the owner of the remote repository
    #[argh(positional)]
    owner: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "info")]
/// Show informations about the remote repository or configuration.
struct RusteaInfo {
    /// print current configuration
    #[argh(switch, short = 'p')]
    print: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// Show feature sets stores in the remote repository
/// or files stored in a feature set.
struct RusteaList {
    /// optional feature set name for content listing
    #[argh(positional)]
    feature_set: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "new")]
/// Create a new feature set in the remote repository
struct RusteaNew {
    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "delete")]
/// Delete a feature set or folders or files within the feature set
struct RusteaDelete {
    /// delete from path recursively
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// delete a script file
    #[argh(switch, short = 's')]
    script: bool,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// the path to a subfolder or file of the feature set
    #[argh(positional)]
    sub_path: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "pull")]
/// Pull a feature set or only the configuration/script files on the local machine.
struct RusteaPull {
    /// deploy only script files
    #[argh(switch, short = 's')]
    script: bool,

    /// deploy only configuration files
    #[argh(switch, short = 'c')]
    config: bool,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// the path to a subfolder or file of the feature set
    #[argh(positional)]
    sub_path: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "push")]
/// Push configuration files or script files to a feature set.
struct RusteaPush {
    /// push a local file to the feature set script folder
    #[argh(switch, short = 's')]
    script: bool,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// the path to a subfolder or file of the feature set
    #[argh(positional)]
    sub_path: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "rename")]
/// Rename in the remote repository a feature set or folders and files in a feature set.
struct RusteaRename {
    /// the path to a subfolder or file of the feature set
    #[argh(option, short = 'p')]
    path: Option<String>,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// the new name of the feature set or folder or file
    #[argh(positional)]
    new_name: String,
}

/// Run the rustea self-updater
fn update() -> Result<self_update::Status, Box<dyn::std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("santifa")
        .repo_name("rustea")
        .bin_name("github")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()?;
    Ok(status)
}

fn main() {
    let rustea: Rustea = argh::from_env();
    let config = match RusteaConfiguration::read_config_file(rustea.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration file not found. Run rustea init --token rustea-devops <repository name> <owner>\nError: {}", e);
            exit(1)
        }
    };

    match rustea.cmd {
        RusteaCmd::Init(init) => {
            match RusteaConfiguration::create_initial_configuration(
                &init.url,
                init.api_token.as_deref(),
                init.token_name.as_deref(),
                &init.repository,
                &init.owner,
            ) {
                Ok(p) => {
                    println!(
                        "Successfully initialized rustea. Configuration path {}",
                        p.display()
                    );
                }
                Err(e) => {
                    eprintln!("Failed to initialize rustea.\nCause: {}", e);
                    exit(1)
                }
            }
        }
        RusteaCmd::Info(info) => {
            if info.print {
                println!("{}", config);
            } else {
                match config.repo.info() {
                    Ok(_) => exit(0),
                    Err(e) => println!("Can not fetch informations. Cause: {}", e),
                }
            }
        }
        RusteaCmd::List(list) => match config.repo.list(list.feature_set.as_deref()) {
            Ok(_) => exit(0),
            Err(e) => println!("Can not fetch informations. Cause: {}", e),
        },
        RusteaCmd::New(new) => {
            match config
                .repo
                .new_feature_set(&new.feature_set, rustea.message.as_deref())
            {
                Ok(_) => exit(0),
                Err(e) => println!("Can not fetch informations. Cause: {}", e),
            }
        }
        RusteaCmd::Delete(delete) => {
            match config.repo.delete(
                &delete.feature_set,
                delete.sub_path.as_deref(),
                delete.script,
                delete.recursive,
                rustea.message.as_deref(),
            ) {
                Ok(_) => println!(
                    "Successfully deleted {} from the feature set {}",
                    delete.sub_path.unwrap_or_default(),
                    delete.feature_set
                ),
                Err(e) => eprintln!(
                    "Failed to delete {} from the feature set {}.\nCause: {}",
                    delete.sub_path.unwrap_or_default(),
                    delete.feature_set,
                    e
                ),
            }
        }
        RusteaCmd::Pull(pull) => {
            match config.repo.pull(
                &pull.feature_set,
                pull.sub_path.as_deref(),
                &config.script_folder,
                pull.script,
                pull.config,
            ) {
                Ok(_) => println!(
                    "Successully pulled files from feature set {}",
                    pull.feature_set
                ),
                Err(e) => eprintln!(
                    "Failed to pull files from feature set {}. Cause {}",
                    pull.feature_set, e
                ),
            }
        }
        RusteaCmd::Push(push) => {
            match config.repo.push(
                &push.feature_set,
                &config.script_folder,
                push.sub_path.as_deref(),
                push.script,
                rustea.message.as_deref(),
            ) {
                Ok(_) => println!(
                    "Successfully pushed files to feature set {}",
                    push.feature_set
                ),
                Err(e) => eprintln!(
                    "Failed to push files to feature set {}. Cause {}",
                    push.feature_set, e
                ),
            }
        }
        RusteaCmd::Rename(rename) => {
            match config.repo.rename(
                &rename.feature_set,
                &rename.new_name,
                rename.path.as_deref(),
                rustea.message.as_deref(),
            ) {
                Ok(_) => println!("Successfully renamed files."),

                Err(e) => eprintln!("Failed to rename files. Error: {}", e),
            }
        }
        RusteaCmd::Update(_) => match update() {
            Ok(status) => println!(
                "Updated successfully, running new version {}",
                status.version()
            ),
            Err(e) => {
                eprintln!("Update failed with reason: {}", e);
                exit(1)
            }
        },
    }
    exit(0);
}
