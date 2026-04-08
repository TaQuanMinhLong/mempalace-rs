use super::*;

#[test]
fn test_wing_person() {
    let wing = Wing::person("kai", vec!["kai".into(), "daughter".into()]);
    assert_eq!(wing.name, "kai");
    assert_eq!(wing.wing_type, WingType::Person);
    assert!(wing.keywords.contains(&"kai".into()));
}

#[test]
fn test_wing_project() {
    let wing = Wing::project("mempalace", vec!["rust".into(), "memory".into()]);
    assert_eq!(wing.name, "mempalace");
    assert_eq!(wing.wing_type, WingType::Project);
}
