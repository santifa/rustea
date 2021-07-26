# rustea

`rustea` is a small cli tool for handling configuration and script files. Thus, it shall be simple
to deploy configurations directly from a Gitea instance and push changed configurations to it.
It doesn't aim to replace full-fledged configuration management system for such an approach
use a tool like Ansible or Chef or something else.

## Overview

`rustea` is a cli tool which connects to the API of a Gitea instances and allows to fetch und push
files. This is mostly done to either deploy or save configuration files or script files.
The (devops) repository has a special format which is explained later. Such that `rustea` deploys
configuration files and script files correctly.

### Repository

The repository needs to contain some more or less obvious informations such that `rustea` can handle files correctly.

Assumptions:
  * A whole repository is used for `rustea`
  * The repository is divided into features (a simple folder) which one want to deploy
  * The configuration files of a feature are stored with its whole original path in this feature folder
  * Script files are stored in the `script` subfolder in a feature folder
  * Script files are stored on the machine in the folder `/usr/local/bin` (to easier distinguish your own files use some common prefix like `nn_` or something else)
  * It assumes that you call `rustea` as root or with sudo if the configuration files are stored in a path unaccessable for our user
  * Authentication is used everytime

Repository Setup:

    Devops Repository:
    |- File_1 <-- ignored
    |- File_2 <-- ignored
    +- feature_set_1/
       +- scripts/ <-- folder containing script files
          |- script_1
          |- script_2
       |- feature_1/ <-- A feature set contains feature folders
       |- feature_2/ <-- The feature folder path is the resulting path in the fs.
    +- feature_set_2/
       |- feature_1/ <-- For example, /etc/postfix/ is stored under mail/etc/postfix/
       |- File_1 <-- ignored

As one can see, files in the root path and in the first level of a feature are ignored. This is
intenionally, to allow readme files within feature sets, for example.


### Workflows

*Prepare rustea*
  * Fetch the binary from somewhere
  * Create an inital configuration in `~/.rustea.toml` with `rustea`:
      * The user has an api token for gitea `rustea init --token <key> <url> <repository> <owner>`
      * The user request a new API key with `rustea init --name <token_name> <url> <repository> <owner>`
          * The user enters username and password when asked and the token is requested 
  * The configuration file is stored under `~/.rustea.toml` by default
      
*Show informations*
  * The user can show informations about the gitea instance and the repository with `rustea info`
  * The user can list all feature-sets in the repository with `rustea list`

*Add a new feature set*
  * The user creates a new feature set with `rustea new <name for the feature set>`
  * This creates a new folder in the devops repository


*Deploy a feature set to the machine*
  * The user deploys a feature set with `rustea pull`
      * For only deploying script files use `rustea pull --script`
      * For only deploying configuration files use `rustea pull --config`
  * `rustea` fetches the content of the feature set 

## Goals

The following goals shall be fullfilled with this little program:
  * single small binary
  * fetch files from a gitea repository with authentication against the Gitea-API
  * Push files from source folders to a gitea repository
  * handle files only
  
## Non-Goals

  * compete with full-fledged configuration management systems
  * regular update of systems and packages
  * repair broken configurations
  * Branches and in deepth Gitea or Git Features (maybe on request)
  * Using plain git and symlinks
  * No authentication is used
  

## Why?

#### Why Gitea?

#### Why Rust?

#### Why not git?

## Tests

The most tests assume that a `rustea.toml` ist located within the project root. Otherwise,
most tests fail.

Run from project the tests root with:

    cargo test
