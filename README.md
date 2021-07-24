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

### Workflows

*Prepare rustea*
  * Fetch the binary from somewhere
  * Create an inital configuration in `~/.rustea.toml` with `rustea`:
      * The user has an api token for gitea `rustea conf --token <key> <url> <repository> <owner>`
      * The user request a new API key with `rustea conf --name <token_name> <url> <repository> <owner>`
          * The user enters username and password when asked and the token is requested 
  * The configuration file is stored under `~/.rustea.toml` by default
      
**

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

