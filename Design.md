# Design document

This is a small design document which provides some kind of short workflows.
These workflows are written from the user perspective and should correspond
to the final command line interface.

### Workflows

The main workflows are written from the cli user perspective.

*Help*
  * The user should get helpful informations with either `--help` or jut `help`
    on the main or any subcommand.

*Prepare rustea and update*
  * Fetch the binary from somewhere
  * Create and initialize a repository on your gitea instance
  * Create an inital configuration in `~/.rustea.toml` with `rustea` or write your own:
    * Run `rustea init <url> <repository> <owner>` to create an example configuration
      * The user can provide a preconfigured token with `--api-token <token>`
      * The user can provide a name for the token with `--token-name <name>`
      * For the creation of a token an initial login of a valid user is required
  * The configuration file is stored under `~/.rustea.toml` by default
  * The update should be seamlessly working with `rustea update`
    * The updater replaces the original binary with a fresh downloaded one if the release version is higher
      
*Show informations*
  * The user can show informations about the gitea instance and the repository with `rustea info`
  * The user can list all feature-sets in the repository with `rustea list`
  * The user can list all script and config files of a feature set with `rustea list <name>`

*Add a new feature set*
  * The user creates a new feature set with `rustea new <feature_set_name>`
    * This creates two empty folders in the remote repository 
      * `<feature_set_name>/.gitkeep` and `<feature_set_name>/scripts/.gitkeep`
  
*Delete a feature set*
  * The user can delete a feature set with `rustea delete <feature_set_name>`
  * The user can further delete subtrees of a feature set with `delete <feature_set_name> <path>`
  * The user can delete script files with `delete -s <feature_set_name> <script_name>`
  * The user can toogle between a normal and a recursive delete

*Add scripts to a feature set*
  * The user adds script files to a feature set with `rustea push -s <feature_set_name> <path>`
  * If the user provides a folder every file is uploaded as script file

*Add config files to a feature set*
  * The user adds configuration files to a feature set with `rustea push <feature_set_name> <path>`
    * The files are canonicolized and stored with its whole path under `<remote_repository>/<feature_set_name>/`

*Exclude files*
  * The user can adjust the global `exclude` variable within the configuration
  * The user should follow the [Rust regex syntax](https://docs.rs/regex/1.5.4/regex/#syntax)
  * This results in files are not pushed to the remote repository but can be pulled

*Update the configuration files*
  * The user may change local configuration files and want to upload the changes
  * The user pushes all configuration files with `rustea push <feature_set_name>`
  * The needed configuration files are determined from the remote repository path 
  * Script files are searched in `/usr/local/bin/`, if the file is located somewhere else use `rustea push -s ...`

*Rename files*
  * the user can rename feature sets with `rustea rename <feature_set_name> <new_name>`
  
*Deploy a feature set to the machine*
  * The user deploys a feature set with `rustea pull <feature_set_name>`
      * For only deploying script files use `rustea pull -s <feature_set_name>`
      * For only deploying configuration files use `rustea pull -c <feature_set_name>`
      * Use `rustea pull <feature_set_name> <path>` for pulling a single file or folder from the feature set
      * The path is the absolute or relative path of the file or folder on the filesystem. 
  * `rustea` fetches the content of the feature set and copies script files to `/usr/local/bin`
    and configration files to their repository path name without the feature set name
  * Local copies are overwritten
  * Sudo is required if the files are copied into filesystem regions where the user has no rights
