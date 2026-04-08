use super::*;

#[test]
fn test_room_detector_creation() {
    let detector = RoomDetector::new();
    assert!(detector.room_keywords.is_empty());
}

#[test]
fn test_detect_from_path() {
    let detector = RoomDetector::new();

    assert_eq!(
        detector.detect_from_path("/home/user/project/frontend/src/index.ts"),
        Some("frontend".to_string())
    );

    assert_eq!(
        detector.detect_from_path("/home/user/project/backend/api/routes.ts"),
        Some("backend".to_string())
    );

    assert_eq!(
        detector.detect_from_path("/home/user/project/docs/readme.md"),
        Some("documentation".to_string())
    );
}

#[test]
fn test_detect_from_content() {
    let mut detector = RoomDetector::new();
    detector.register_room(Room::new(
        "frontend",
        "wing_code",
        vec!["button".to_string(), "component".to_string()],
    ));

    let content = "The button component needs to be updated";
    assert_eq!(
        detector.detect_from_content(content),
        Some("frontend".to_string())
    );
}

#[test]
fn test_register_room() {
    let mut detector = RoomDetector::new();
    detector.register_room(Room::new(
        "testing",
        "wing_code",
        vec!["test".to_string(), "spec".to_string()],
    ));

    assert!(detector.room_keywords.contains_key("testing"));
    assert_eq!(
        detector.room_keywords.get("testing"),
        Some(&vec!["test".to_string(), "spec".to_string()])
    );
}

#[test]
fn test_register_folder_mapping() {
    let mut detector = RoomDetector::new();
    detector.register_folder_mapping("src_client", "frontend");

    let result = detector.detect_from_path("/home/user/src_client/components/Button.tsx");
    assert_eq!(result, Some("frontend".to_string()));
}

#[test]
fn test_detected_room_to_room() {
    let detected = DetectedRoom {
        name: "testing".to_string(),
        description: "Test files".to_string(),
        keywords: vec!["test".to_string(), "spec".to_string()],
    };

    let room = detected.to_room("wing_code");
    assert_eq!(room.name, "testing");
    assert_eq!(room.wing, "wing_code");
    assert_eq!(room.keywords, vec!["test", "spec"]);
}

#[test]
fn test_should_skip_dir() {
    assert!(RoomDetector::should_skip_dir(".git"));
    assert!(RoomDetector::should_skip_dir("node_modules"));
    assert!(RoomDetector::should_skip_dir("__pycache__"));
    assert!(!RoomDetector::should_skip_dir("src"));
    assert!(!RoomDetector::should_skip_dir("my_frontend"));
}

#[test]
fn test_get_room_from_folder() {
    assert_eq!(
        RoomDetector::get_room_from_folder("frontend"),
        Some("frontend".to_string())
    );
    assert_eq!(
        RoomDetector::get_room_from_folder("front_end"),
        Some("frontend".to_string())
    );
    assert_eq!(
        RoomDetector::get_room_from_folder("back-end"),
        Some("backend".to_string())
    );
    assert_eq!(
        RoomDetector::get_room_from_folder("docs"),
        Some("documentation".to_string())
    );
    assert_eq!(
        RoomDetector::get_room_from_folder("designs"),
        Some("design".to_string())
    );
    assert_eq!(RoomDetector::get_room_from_folder("unknown_folder"), None);
}

#[test]
fn test_registered_rooms() {
    let mut detector = RoomDetector::new();
    detector.register_room(Room::new("room1", "wing1", vec![]));
    detector.register_folder_mapping("folder1", "room2");

    let rooms = detector.registered_rooms();
    assert!(rooms.contains(&"room1".to_string()));
    assert!(rooms.contains(&"room2".to_string()));
}
