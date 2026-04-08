use super::*;

#[test]
fn test_wing_name_from_dir_simple() {
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/koboldcpp")),
        "wing_koboldcpp"
    );
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/projects/my-app")),
        "wing_my-app"
    );
}

#[test]
fn test_wing_name_slugifies() {
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/my project")),
        "wing_my-project"
    );
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/KoboldCpp")),
        "wing_koboldcpp"
    );
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/My Project!")),
        "wing_my-project"
    );
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("/home/me/a  b")),
        "wing_a-b"
    );
}

#[test]
fn test_wing_name_from_dir_dot() {
    let result = wing_name_from_dir(&PathBuf::from("."));
    assert!(
        result.starts_with("wing_"),
        "expected wing_<dirname>, got {}",
        result
    );
    assert_ne!(result, "wing_general");
}

#[test]
fn test_wing_name_from_dir_dotdot() {
    let result = wing_name_from_dir(&PathBuf::from(".."));
    assert!(
        result.starts_with("wing_"),
        "expected wing_<dirname>, got {}",
        result
    );
    assert_ne!(result, "wing_general");
}

#[test]
fn test_wing_name_from_dir_relative() {
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("./myproject")),
        "wing_myproject"
    );
    assert_eq!(
        wing_name_from_dir(&PathBuf::from("../someproject")),
        "wing_someproject"
    );
}

#[test]
fn test_wing_name_from_dir_empty_string() {
    let result = wing_name_from_dir(&PathBuf::from(""));
    assert!(
        result.starts_with("wing_"),
        "expected wing_<dirname>, got {}",
        result
    );
}
