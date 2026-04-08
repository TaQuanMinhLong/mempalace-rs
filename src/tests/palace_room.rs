use super::*;

#[test]
fn test_room_creation() {
    let room = Room::new(
        "Auth Migration",
        "wing_code",
        vec!["auth".into(), "jwt".into()],
    );
    assert_eq!(room.name, "Auth Migration");
    assert_eq!(room.wing, "wing_code");
}

#[test]
fn test_slugify() {
    assert_eq!(Room::slugify("Auth Migration"), "auth-migration");
    assert_eq!(Room::slugify("Simple"), "simple");
    assert_eq!(Room::slugify("Multiple   Spaces"), "multiple-spaces");
}
