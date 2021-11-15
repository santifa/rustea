/// For these integration tests provide a `rustea.toml`
/// file in the projects root.
/// Use an empty repository for testing.
use rustea::gitea::GiteaClient;

fn load_dev_conf() -> GiteaClient {
    let conf = rustea::RusteaConfiguration::read_config_file(Some("rustea.toml")).unwrap();
    GiteaClient::new(
        &conf.repo.url,
        Some(&conf.repo.api_token),
        None,
        &conf.repo.repository,
        &conf.repo.owner,
    )
    .unwrap()
}

#[test]
fn test_get_gitea_version() {
    let client = load_dev_conf();
    let version = client.get_gitea_version();
    println!("{:#?}", version);
    assert!(version.is_ok());
    assert_eq!("1.14.4", version.unwrap().version)
}

#[test]
fn test_get_repository() {
    let client = load_dev_conf();
    let repository = client.get_repository_information();
    println!("{:#?}", repository);
    assert!(repository.is_ok());
}

// #[test]
// fn test_get_empty_feature_sets() {
//     let client = load_dev_conf();
//     let feature_sets = client.get_repository_features();
//     println!("{:#?}", feature_sets);
//     assert!(feature_sets.is_ok());
//     let fs = feature_sets.unwrap();
//     assert_eq!(0, fs.content.len());
// }

// #[test]
// fn test_feature_set_lifecycle() {
//     let client = load_dev_conf();
//     let res = client.create_new_feature_set("test-feature");
//     println!("Creation: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.check_feature_set_exists("test-feature");
//     println!("Existence: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.check_file_exists("test-feature", ".gitkeep");
//     println!("Existence: {:#?}", res);
//     assert!(res);

//     let feature_sets = client.get_repository_features();
//     println!("Feature Sets: {:#?}", feature_sets);
//     assert!(feature_sets.is_ok());
//     let fs = feature_sets.unwrap();
//     println!("Feature Sets: {:#?}", fs);
//     assert_eq!(1, fs.content.len());

//     let res = client.delete_file_or_folder("test-feature/.gitkeep", false);
//     println!("Deletion: {:#?}", res);
//     assert!(res.is_ok());
//     panic!();
// }

// #[test]
// fn test_feature_not_exists() {
//     let client = load_dev_conf();
//     let res = client.check_feature_set_exists("README.md");
//     println!("{:#?}", res);
//     assert!(!res.unwrap());
//     let res = client.check_feature_set_exists("README.dd");
//     println!("{:#?}", res);
//     assert!(!res.unwrap());
// }

// #[test]
// fn test_create_existing_feature_set() {
//     let client = load_dev_conf();
//     let res = client.create_new_feature_set("test-feature");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.create_new_feature_set("test-feature");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.delete_feature_set("test-feature");
//     println!("{:#?}", res);
//     assert!(res.is_ok());
// }

// #[test]
// fn test_create_file() {
//     let client = load_dev_conf();
//     let res = client.create_new_feature_set("test-feature");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.create_file("test-feature", "test", "ping");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.check_file_exists("test-feature", "test");
//     println!("{:#?}", res);
//     assert!(res);

//     let res = client.delete_file_or_folder("test-feature/.gitkeep", false);
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.delete_file_or_folder("test-feature/test", false);
//     println!("{:#?}", res);
//     assert!(res.is_ok());
// }

// #[test]
// fn test_update_file() {
//     let client = load_dev_conf();
//     let res = client.create_new_feature_set("test-feature");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.create_file("test-feature", "test", "ping");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.check_file_exists("test-feature", "test");
//     println!("{:#?}", res);
//     assert!(res);

//     let res = client.create_or_update_file("test-feature", "test", "ping2");
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.delete_file_or_folder("test-feature/.gitkeep", false);
//     println!("{:#?}", res);
//     assert!(res.is_ok());

//     let res = client.delete_file_or_folder("test-feature/test", false);
//     println!("{:#?}", res);
//     assert!(res.is_ok());
// }

// #[test]
// fn test_delete_feature_set() {
//     let client = load_dev_conf();
//     let res = client.create_new_feature_set("test-feature");
//     println!("Creating: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.create_or_update_file("test-feature", "test", "ping2");
//     println!("Updating: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.create_or_update_file("test-feature", "testing/feature", "ping2");
//     println!("Updating: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.get_file_or_folder("test-feature", None);
//     println!("Listing: {:#?}", res);
//     assert!(res.is_ok());

//     let res = client.delete_file_or_folder("test-feature", true);
//     println!("Deletion: {:#?}", res);
//     assert!(res.is_ok());
//     panic!();
// }

// #[test]
// fn test_add_config_to_feature_set() {
//     assert!(false, "To implement");
// }

// #[test]
// fn test_push_config() {
//     assert!(false, "To implement");
// }

// #[test]
// fn test_pull_config() {
//     assert!(false, "To implement");
// }
