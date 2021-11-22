# rustea

`rustea` is a small cli tool for handling configuration and script files. Thus, it shall be simple
to pull or push configurations directly from a Gitea instance. It doesn't aim to replace full-fledged
configuration management system. If your looking for such an approach use a tool like Ansible 
or Chef or something else like that.

## Overview

`rustea` uses a git like semantics where configurations are either pushed to some Gitea instance
or pulled to the local machine and copied to the correct place. It distinguishes between simple
files and __script files__ which are simply executable files stored in a special location.
As a remote store for the files a Gitea server with an enabled API is used.

**Semantics***  
`rustea` uses the following semantics:  
  * **Remote repository**: The main git repository hosted on a gitea instance running anywhere which is
  accessible via its API. `rustea` only access one remote repository at a time.
  * **Feature set**: A remote repository contains multiple feature sets which can provide different configuration
  or script files.
  * **Pull**: Describes the action of downloading configuration and/or script files from a feature set
  and store them on the local machine.
  * **Push**: Describes the action of uploading local configuration and/or script files to a feature set in
  the remote repository.

#### Non-Goals

  * compete with full-fledged configuration management systems
  * regular update of operating systems and distribution packages
  * repair broken configurations
  * Branches and in deepth Gitea or Git Features (maybe on request)

### Why?

The main idea behind `rustea` is to have a single static binary for configuration or feature management
of *nix machines. It shall allow version control but without the need of having a local `git` installation
which is quite large. Most configuration systems depend on open ports or `ssh` installed and configured on
the target machine.

Gitea is a lightwight and fast Github and Gitlab alternative written as a single go binary. It has an extensive
API with a good __swagger__ documentation. Alternative backends such as Github are also possible (maybe on request).

### Repository

`rustea` is build around the idea to use a single repository for configuration managment. The repository contains
so called feature sets which defines configuration files and/or script files for a single feature. For example,
a feature set can be `php` with the only configuration file `/etc/php/php.ini`. A more complex example can be the
feature `mail-server` which contains dozen of configuration files for `postfix`, `dovecot`, `postgres`, `rspamd` and
self-made __script files__ for adding users or domains to the mail server.

The following is an example repository:

    Devops Repository:
    |- File_1 <-- ignored
    |- File_2 <-- ignored
    +- feature_set_1/ <- First recognized folder/file
       +- scripts/ <-- folder containing script files
          |- script_1
          |- script_2
       |- feature_1/ <-- A feature set contains feature folders
       |- feature_2/ <-- The feature folder path is the resulting path in the fs.
    +- feature_set_2/
       |- feature_1/ <-- For example, /etc/postfix/ is stored remotely under mail/etc/postfix/
       |- File_1 <-- Is deployed under /
       +- home/
          +- user/
             +- .zshrc <-- Is deployed under /home/user/.zshrc

As one can see, files in the root directory are ignored. A feature set lives within a folder denoted by
the name of the feature set. Within a feature set __script files__ are placed directly in the folder `scripts`.
Configuration files are placed in the feature set with its full path. This enables rustea to simply copy
the files directly to the appropriate position on a local machine. The drawback is that differences between
linux distributions or between operating systems are not handled. Files directly stored in a feature set
are placed under `/`.

The following list gives some assumptions while developing `rustea`:
  * `rustea` uses a single repository
  * feature sets are stored in a folder by their name
  * __script files__ are stored in `<feature_set_name>/scripts/`
  * configuration files are canonicolized and stored in `<feature_set_name>/full/path/to/config/file`
  * The destination of __script files__ is configurable
  * Token authentication is used for every request
  * Pull operations are collective operations for all __script files__ and/or configuration files
  * `rustea` is called as root or with sudo if the configuration files are stored in sensible locations
  * `.gitkeep` is used to store empty feature sets and script folders

An example for the main configuration which is stored under `~/.rustea.toml`:

    script_folder = '/etc/local/bin' <-- Local folder for script files
    exclude = '\.git$' <-- Files an folders excluded

    [repo]
    url = 'https://git.rtzptz.xyz' <-- Base url to the gitea instance without trailing /
    repository = 'rustea-devops' <-- Repository name
    owner = 'Juerges' <-- Repository owner
    api_token = 'xxxxx' <-- Provided or created by the initialization of rustea
    author = "Henrik Jürges" <-- Should match with some Username but everything is allowed
    email = "example@rtzptz.xyz" <-- Change after initialization
    
The API token can be requested while initializing `rustea` which also creates the initial configuration.
The name and email address are used for commiting.

## Installation and Usage

Either grab a pre-build copy:

    curl -L https://github.com/santifa/rustea/releases/download/v0.1.4/rustea-min > /usr/local/bin/rustea

or build `rustea` on your own:

    git clone https://github.com/santifa/rustea.git
    cd rustea
    cargo build --release

Now you can create a new repository within in your Gitea Instance. 
__!!! Be aware that you must initialize your repository with some README.md or something else.
An empty repository refuses to add new files via API !!!__

Afterwards, you can either create the `~/.rustea.toml` by yourself or run `rustea init -n <TOKEN-NAME> <URL> <REPO> <OWNER>`.

`rustea` uses some optimization for the binary size: [[Ref]](https://arusahni.net/blog/2020/03/optimizing-rust-binary-size.html), [[Ref]](https://github.com/johnthagen/min-sized-rust)

  * [x] build in release mode
  * [x] Optimization for size with  `opt-level = "s"`
  * [x] Link-time Optimization with `lto = true`
  * [x] Reduce parallel code building with `codegen-units = 1`
  * [x] Abort on panic instead of unwind the stack with `panic = 'abort'`
  * [ ] Use xargo for `std`?
  * [ ] Remove `libstd`?
  * [x] Strip symbols with `cargo-strip` 
  * [x] Compress the binary with upx

The last two options can lead to insufficient error messsages and virus scanner alert. 
Thus, two version are provided with and without striped symbols and compression.

## Development and Contribution

This crate and tool is still young so feature requests and issues are welcome.
Feel free to open a pull requests if you implemented a new feature or closed something from the
todo list. Open a new issues if you found bugs or want to provide notes on the code.

### Tests

The test can be run with `cargo test`. The integration tests assume a configured `rustea.toml` in
the project root which points to an empty remote repository.

### Todo's

A small list of features that came in my mind:
  * [x] Commit messages from `rustea`
  * [x] Use binary format for reading files
  * [x] self-updater
  * [+] Rename features set, files or folder on the remote repository
    * [x] feature sets
    * [ ] files
    * [ ] folders
  * [ ] set symlink files (e.g. for cron-jobs)
  * [x] better terminal support (better display of tables)
  * [ ] installing packages, distribution agnostic?
  * [x] Ignore specific files like `.git` (Follows the [Rust regex syntax](https://docs.rs/regex/1.5.4/regex/#syntax))
  * [x] Pull single configuration or script files from a feature set
        (Doesn't distinguishes between similar named pathes like `/test` and `/testtest` when only `test` is given)
  * [ ] Provide other backends like Gitlab or Github
  * [ ] Show diff between the local and remote configuration
  * [ ] More extensives tests
  * [ ] feature set and local folder diff
  * [x] Scripts should be executable (this is set explicit to 751)
  * [x] Replace (https://docs.rs/reqwest/0.11.4/reqwest/index.html)[`reqwest`] with something smaller; (https://docs.rs/curl/0.4.38/curl/index.html)[curl-bindings], (https://github.com/algesten/ureq)[ureq]

### License

    rustea is a small cli tool to interact with git repositories hosted 
    by Gitea Instances. Copyright (C) 2021  Henrik Jürges (juerges.henrik@gmail.com)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program. If not, see <https://www.gnu.org/licenses/>.
