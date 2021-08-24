/// These integration tests are used for testing
/// the configuration loading and assumes a `rustea.toml`
/// in the projects root.
use rustea::Configuration;
use std::{
    fs::{self},
    path::PathBuf,
};

const DEV_FILE: Option<&str> = Some("rustea.toml");

#[test]
fn test_parse_config() {
    let conf_result = Configuration::read_config_file(DEV_FILE);
    assert!(conf_result.is_ok());
    let conf = conf_result.unwrap();
    assert_eq!(conf.script_folder, "test_bin");
    assert_eq!(conf.repo.owner, "Juerges");
    assert_eq!(conf.repo.email, "test@test.de");
    assert_eq!(conf.repo.url, "https://git.cobios.de");
    assert_eq!(conf.repo.repository, "rustea-devops");
    assert_eq!(conf.repo.author, "Testuser");
}

#[test]
fn test_no_config_file() {
    let conf_result = Configuration::read_config_file(Some(""));
    println!("{:#?}", conf_result);
    assert!(conf_result.is_err());
}

#[test]
fn test_write_config_file() {
    let conf_result = Configuration::read_config_file(DEV_FILE).unwrap();
    let path = PathBuf::from("test_bin/rustea.toml");
    assert!(!path.exists());
    conf_result.write_config_file(&path);
    assert!(path.exists());
    fs::remove_file(&path);
}
