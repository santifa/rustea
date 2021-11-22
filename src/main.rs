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
extern crate rpassword;
extern crate serde;
extern crate serde_json;
extern crate tabwriter;
extern crate toml;
extern crate ureq;
extern crate regex;

use argh::FromArgs;
use rustea::{updater::Updater, RemoteRepository, RusteaConfiguration};
use std::process::exit;

#[derive(FromArgs, PartialEq, Debug)]
/// A simple cli configuration management which uses gitea as backend.
struct Rustea {
    /// provide a custom configuration file
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// a commit message used for changes in he remote repository
    #[argh(option, short = 'm')]
    message: Option<String>,

    /// the action which rustea executes
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
struct RusteaUpdate {
    /// fetch the minified version of rustea
    #[argh(switch, short = 'm')]
    minified: bool,
}

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
/// Show informations about rustea and the remote repository.
struct RusteaInfo {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "list")]
/// Show feature sets stores in the remote repository.
struct RusteaList {
    /// provide a feature set name for listing its content
    #[argh(positional)]
    feature_set: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "new")]
/// Create a new feature set in the remote repository.
struct RusteaNew {
    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "delete")]
/// Delete a feature set, files or folders from the remote repository.
struct RusteaDelete {
    /// delete from path recursively
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// delete a script file within a feature set
    #[argh(switch, short = 's')]
    script: bool,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// an optional path within a feature set which should be deleted
    #[argh(positional)]
    sub_path: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "pull")]
/// Pull a feature set or parts of it to the local machine.
struct RusteaPull {
    /// pull only script files
    #[argh(switch, short = 's')]
    script: bool,

    /// pull only configuration files
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
    /// push a local file to the script folder of a feature set
    #[argh(switch, short = 's')]
    script: bool,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// push a path to the feature set
    #[argh(positional)]
    sub_path: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "rename")]
/// Rename a feature set, file or folder in the remote repository.
struct RusteaRename {
    /// an optional path to a file or folder which should be renamed
    #[argh(option, short = 'p')]
    path: Option<String>,

    /// the name of the feature set
    #[argh(positional)]
    feature_set: String,

    /// the new name for the feature set or path
    #[argh(positional)]
    new_name: String,
}

fn main() {
    let rustea: Rustea = argh::from_env();

    if let RusteaCmd::Init(ref init) = rustea.cmd {
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

    let config = match RusteaConfiguration::read_config_file(rustea.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration file not found. Run rustea init --token rustea-devops <repository name> <owner>\nError: {}", e);
            exit(1)
        }
    };
    let remote_repository = match RemoteRepository::new(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Could not create client for remote repository: {}", e);
            exit(1)
        }
    };

    let res = match rustea.cmd {
        RusteaCmd::Init(_) => Ok("Already initialized".to_string()),
        RusteaCmd::Info(_) => Ok(format!("{}", remote_repository)),
        RusteaCmd::List(list) => remote_repository.list(list.feature_set),
        RusteaCmd::New(new) => remote_repository.new_feature_set(&new.feature_set, rustea.message),
        RusteaCmd::Delete(delete) => remote_repository.delete(
            &delete.feature_set,
            delete.sub_path,
            delete.script,
            delete.recursive,
            rustea.message,
        ),
        RusteaCmd::Pull(pull) => {
            remote_repository.pull(&pull.feature_set, pull.sub_path, pull.script, pull.config)
        }
        RusteaCmd::Push(push) => remote_repository.push(
            &push.feature_set,
            push.sub_path,
            push.script,
            rustea.message,
        ),
        RusteaCmd::Rename(rename) => remote_repository.rename(
            &rename.feature_set,
            &rename.new_name,
            rename.path,
            rustea.message,
        ),
        RusteaCmd::Update(update) => Updater::new().and_then(|u| u.update(update.minified)),
    };

    match res {
        Ok(s) => println!("{}", s),
        Err(e) => {
            eprintln!("{}", e);
            exit(1)
        }
    }
    exit(0);
}
