//! Room detection - port from Python room_detector_local.py
//!
//! Two ways to define rooms without calling any AI:
//!   1. Auto-detect from folder structure (zero config)
//!   2. Define manually via register_room()

use crate::palace::Room;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Folder-to-room mapping
const FOLDER_ROOM_MAP: &[(&str, &str)] = &[
    ("frontend", "frontend"),
    ("front-end", "frontend"),
    ("front_end", "frontend"),
    ("client", "frontend"),
    ("ui", "frontend"),
    ("views", "frontend"),
    ("components", "frontend"),
    ("pages", "frontend"),
    ("backend", "backend"),
    ("back-end", "backend"),
    ("back_end", "backend"),
    ("server", "backend"),
    ("api", "backend"),
    ("routes", "backend"),
    ("services", "backend"),
    ("controllers", "backend"),
    ("models", "backend"),
    ("database", "backend"),
    ("db", "backend"),
    ("docs", "documentation"),
    ("doc", "documentation"),
    ("documentation", "documentation"),
    ("wiki", "documentation"),
    ("readme", "documentation"),
    ("notes", "documentation"),
    ("design", "design"),
    ("designs", "design"),
    ("mockups", "design"),
    ("wireframes", "design"),
    ("assets", "design"),
    ("storyboard", "design"),
    ("costs", "costs"),
    ("cost", "costs"),
    ("budget", "costs"),
    ("finance", "costs"),
    ("financial", "costs"),
    ("pricing", "costs"),
    ("invoices", "costs"),
    ("accounting", "costs"),
    ("meetings", "meetings"),
    ("meeting", "meetings"),
    ("calls", "meetings"),
    ("meeting_notes", "meetings"),
    ("standup", "meetings"),
    ("minutes", "meetings"),
    ("team", "team"),
    ("staff", "team"),
    ("hr", "team"),
    ("hiring", "team"),
    ("employees", "team"),
    ("people", "team"),
    ("research", "research"),
    ("references", "research"),
    ("reading", "research"),
    ("papers", "research"),
    ("planning", "planning"),
    ("roadmap", "planning"),
    ("strategy", "planning"),
    ("specs", "planning"),
    ("requirements", "planning"),
    ("tests", "testing"),
    ("test", "testing"),
    ("testing", "testing"),
    ("qa", "testing"),
    ("scripts", "scripts"),
    ("tools", "scripts"),
    ("utils", "scripts"),
    ("config", "configuration"),
    ("configs", "configuration"),
    ("settings", "configuration"),
    ("infrastructure", "configuration"),
    ("infra", "configuration"),
    ("deploy", "configuration"),
];

/// Directories to skip during room detection
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    ".next",
    "coverage",
    ".mempalace",
];

/// Room detector for local setup (no API required)
#[derive(Debug, Clone)]
pub struct RoomDetector {
    /// Room name to keywords mapping
    room_keywords: HashMap<String, Vec<String>>,
    /// Custom folder-to-room mappings
    custom_folder_map: HashMap<String, String>,
}

impl RoomDetector {
    /// Create a new room detector
    pub fn new() -> Self {
        Self {
            room_keywords: HashMap::new(),
            custom_folder_map: HashMap::new(),
        }
    }

    /// Detect room from file path and content keywords
    pub fn detect_room(&self, file_path: &str, content: &str) -> Option<String> {
        // First try to detect from folder structure
        if let Some(room) = self.detect_from_path(file_path) {
            return Some(room);
        }

        // Fall back to content keyword detection
        self.detect_from_content(content)
    }

    /// Detect room from file path
    fn detect_from_path(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);

        // Check each component of the path
        let mut current = path;
        while let Some(parent) = current.parent() {
            if let Some(folder_name) = parent.file_name() {
                let folder_str = folder_name.to_string_lossy().to_lowercase();
                let folder_clean = folder_str.replace("-", "_").replace(" ", "_");

                // Check custom folder map first
                if let Some(room) = self.custom_folder_map.get(&folder_clean) {
                    return Some(room.clone());
                }

                // Check built-in folder map
                if let Some(room) = Self::get_room_from_folder(&folder_clean) {
                    return Some(room);
                }
            }
            current = parent;
        }

        None
    }

    /// Get room from folder name using built-in mapping
    fn get_room_from_folder(folder: &str) -> Option<String> {
        for (key, room) in FOLDER_ROOM_MAP {
            if folder.contains(key) {
                return Some(room.to_string());
            }
        }
        None
    }

    /// Detect room from content keywords
    fn detect_from_content(&self, content: &str) -> Option<String> {
        let content_lower = content.to_lowercase();

        // Check against registered room keywords
        let mut best_match: Option<(String, usize)> = None;

        for (room, keywords) in &self.room_keywords {
            let match_count = keywords
                .iter()
                .filter(|kw| content_lower.contains(&kw.to_lowercase()))
                .count();

            if match_count > 0 {
                match best_match {
                    None => best_match = Some((room.clone(), match_count)),
                    Some((_, count)) if match_count > count => {
                        best_match = Some((room.clone(), match_count));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(room, _)| room)
    }

    /// Register a room with keywords
    pub fn register_room(&mut self, room: Room) {
        self.room_keywords.insert(room.name.clone(), room.keywords);
    }

    /// Register a folder-to-room mapping
    pub fn register_folder_mapping(&mut self, folder: &str, room: &str) {
        self.custom_folder_map
            .insert(folder.to_lowercase(), room.to_string());
    }

    /// Detect rooms from folder structure of a project directory
    pub fn detect_rooms_from_folders(&self, project_dir: &str) -> Vec<DetectedRoom> {
        let project_path = Path::new(project_dir);
        let mut found_rooms: HashMap<String, String> = HashMap::new();

        // Check top-level directories first
        if let Ok(entries) = std::fs::read_dir(project_path) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !Self::should_skip_dir(&name_str) {
                        let name_lower = name_str.to_lowercase().replace("-", "_");
                        if let Some(room) = Self::get_room_from_folder(&name_lower) {
                            if !found_rooms.contains_key(&room) {
                                found_rooms.insert(room.clone(), name_str.to_string());
                            }
                        } else if name_str.len() > 2
                            && name_str
                                .chars()
                                .next()
                                .map(|c| c.is_alphabetic())
                                .unwrap_or(false)
                        {
                            // Use folder name directly as room if it looks like a valid identifier
                            let clean = name_lower.replace("-", "_").replace(" ", "_");
                            if !found_rooms.contains_key(&clean) {
                                found_rooms.insert(clean.clone(), name_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Walk one level deeper for nested patterns
        if let Ok(entries) = std::fs::read_dir(project_path) {
            for entry in entries.flatten() {
                if entry.path().is_dir()
                    && !Self::should_skip_dir(&entry.file_name().to_string_lossy())
                {
                    if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            if sub_entry.path().is_dir() {
                                let sub_name = sub_entry.file_name();
                                let sub_name_str = sub_name.to_string_lossy();
                                if !Self::should_skip_dir(&sub_name_str) {
                                    let sub_lower = sub_name_str.to_lowercase().replace("-", "_");
                                    if let Some(room) = Self::get_room_from_folder(&sub_lower) {
                                        if !found_rooms.contains_key(&room) {
                                            found_rooms
                                                .insert(room.clone(), sub_name_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build room list
        let mut rooms: Vec<DetectedRoom> = found_rooms
            .into_iter()
            .map(|(room, original)| DetectedRoom {
                name: room.clone(),
                description: format!("Files from {}/", original),
                keywords: vec![room],
            })
            .collect();

        // Always add "general" as fallback
        if !rooms.iter().any(|r| r.name == "general") {
            rooms.push(DetectedRoom {
                name: "general".to_string(),
                description: "Files that don't fit other rooms".to_string(),
                keywords: vec![],
            });
        }

        rooms
    }

    /// Detect rooms from filename patterns
    pub fn detect_rooms_from_files(&self, project_dir: &str) -> Vec<DetectedRoom> {
        let project_path = Path::new(project_dir);
        let mut keyword_counts: HashMap<String, usize> = HashMap::new();

        let walker = ignore::WalkBuilder::new(project_path).hidden(true).build();

        for entry in walker.flatten() {
            if entry.path().is_file() {
                let filename = entry.file_name().to_string_lossy().to_lowercase();
                let filename_clean = filename.replace("-", "_").replace(" ", "_");

                for (keyword, room) in FOLDER_ROOM_MAP {
                    if filename_clean.contains(keyword) {
                        *keyword_counts.entry(room.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Return rooms that appear more than twice
        let mut rooms: Vec<_> = keyword_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(room, _)| DetectedRoom {
                name: room.clone(),
                description: format!("Files related to {}", room),
                keywords: vec![room],
            })
            .collect();

        rooms.sort_by_key(|r| Reverse(r.keywords.len()));

        if rooms.is_empty() {
            rooms.push(DetectedRoom {
                name: "general".to_string(),
                description: "All project files".to_string(),
                keywords: vec![],
            });
        }

        rooms.truncate(6);
        rooms
    }

    /// Check if a directory should be skipped
    fn should_skip_dir(name: &str) -> bool {
        SKIP_DIRS.contains(&name) || name.starts_with('.')
    }

    /// Get all registered room names
    pub fn registered_rooms(&self) -> Vec<String> {
        let mut rooms: HashSet<String> = self.room_keywords.keys().cloned().collect();
        rooms.extend(self.custom_folder_map.values().cloned());
        rooms.into_iter().collect()
    }
}

impl Default for RoomDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Detected room from folder/file analysis
#[derive(Debug, Clone)]
pub struct DetectedRoom {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
}

impl DetectedRoom {
    /// Convert to a Room object
    pub fn to_room(&self, wing: &str) -> Room {
        Room::new(&self.name, wing, self.keywords.clone())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_detector_creation() {
        let detector = RoomDetector::new();
        assert!(detector.room_keywords.is_empty());
    }

    #[test]
    fn test_detect_from_path() {
        let detector = RoomDetector::new();

        // Test frontend detection
        assert_eq!(
            detector.detect_from_path("/home/user/project/frontend/src/index.ts"),
            Some("frontend".to_string())
        );

        // Test backend detection
        assert_eq!(
            detector.detect_from_path("/home/user/project/backend/api/routes.ts"),
            Some("backend".to_string())
        );

        // Test documentation detection
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
}
