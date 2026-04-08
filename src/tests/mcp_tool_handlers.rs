use crate::mcp::tools;
use crate::mcp_tools::McpTestHarness;
use crate::palace::IngestMode;
use futures::FutureExt;
use rmcp::model::RawContent;
use serde_json::Value;

fn parse_json(result: rmcp::model::CallToolResult) -> Value {
    let text = result
        .content
        .into_iter()
        .find_map(|content| match content.raw {
            RawContent::Text(text) => Some(text.text),
            _ => None,
        })
        .expect("tool result should contain text content");

    serde_json::from_str(&text).expect("tool result should be valid JSON")
}

#[tokio::test]
async fn catalog_tools_return_expected_counts() {
    let harness = McpTestHarness::new(true);
    harness
        .add_drawer(
            "drawer-1",
            "memory systems design notes",
            "wing_memory",
            "architecture",
            "notes/a.txt",
            "tester",
            IngestMode::Projects,
        )
        .await;
    harness
        .add_drawer(
            "drawer-2",
            "memory retrieval checklist",
            "wing_memory",
            "operations",
            "notes/b.txt",
            "tester",
            IngestMode::Projects,
        )
        .await;
    harness
        .add_triple("triple-1", "memory", "relates_to", "palace", None, None)
        .await;

    let status = parse_json(
        tools::catalog::status(&harness.server)
            .await
            .expect("status should succeed"),
    );
    assert_eq!(status["total_drawers"], 2);
    assert_eq!(status["total_triples"], 1);
    assert_eq!(status["wings"]["wing_memory"], 2);

    let wings = parse_json(
        tools::catalog::list_wings(&harness.server)
            .await
            .expect("list_wings should succeed"),
    );
    assert_eq!(wings["wings"].as_array().expect("wings array").len(), 1);
    assert_eq!(wings["wings"][0]["name"], "wing_memory");

    let rooms = parse_json(
        tools::catalog::list_rooms(
            &harness.server,
            &tools::catalog::ListRoomsArgs {
                wing: Some("wing_memory".to_string()),
            },
        )
        .await
        .expect("list_rooms should succeed"),
    );
    assert_eq!(rooms["wing"], "wing_memory");
    assert_eq!(rooms["rooms"].as_array().expect("rooms array").len(), 2);

    let taxonomy = parse_json(
        tools::catalog::get_taxonomy(&harness.server)
            .await
            .expect("get_taxonomy should succeed"),
    );
    assert_eq!(taxonomy["taxonomy"]["wing_memory"]["architecture"], 1);
    assert_eq!(taxonomy["taxonomy"]["wing_memory"]["operations"], 1);

    let spec = parse_json(tools::catalog::get_aaak_spec().expect("get_aaak_spec should succeed"));
    assert!(spec["aaak_spec"]
        .as_str()
        .expect("spec string")
        .contains("AAAK"));
}

#[tokio::test]
async fn search_tools_return_results_inside_runtime() {
    let harness = McpTestHarness::new(false);
    harness
        .add_drawer(
            "drawer-1",
            "memory palace retrieval workflow for project context",
            "wing_memory",
            "search",
            "notes/search.txt",
            "tester",
            IngestMode::Projects,
        )
        .await;

    let search_args = tools::search::SearchArgs {
        query: "memory".to_string(),
        limit: Some(5),
        wing: Some("wing_memory".to_string()),
        room: None,
    };
    let search_result = parse_json(
        tools::search::search(&harness.server, &search_args)
            .await
            .expect("search should succeed"),
    );
    assert_eq!(search_result["count"], 1);
    assert_eq!(search_result["results"][0]["wing"], "wing_memory");

    let duplicate_args = tools::search::CheckDuplicateArgs {
        content: "memory palace retrieval workflow".to_string(),
        threshold: Some(0.1),
    };
    let duplicate_result = parse_json(
        tools::search::check_duplicate(&harness.server, &duplicate_args)
            .await
            .expect("check_duplicate should succeed"),
    );
    assert_eq!(duplicate_result["is_duplicate"], true);
    assert_eq!(
        duplicate_result["matches"]
            .as_array()
            .expect("matches array")
            .len(),
        1
    );
}

#[tokio::test]
async fn graph_tools_return_connections_inside_runtime() {
    let harness = McpTestHarness::new(false);
    harness
        .add_drawer(
            "drawer-1",
            "memory design in alpha",
            "wing_alpha",
            "shared-room",
            "alpha/shared.txt",
            "tester",
            IngestMode::Projects,
        )
        .await;
    harness
        .add_drawer(
            "drawer-2",
            "memory design in beta",
            "wing_beta",
            "shared-room",
            "beta/shared.txt",
            "tester",
            IngestMode::Projects,
        )
        .await;

    let traverse_args = tools::graph::TraverseArgs {
        start_room: "shared-room".to_string(),
        max_hops: Some(2),
    };
    let traverse_result = parse_json(
        tools::graph::traverse(&harness.server, &traverse_args)
            .await
            .expect("traverse should succeed"),
    );
    assert_eq!(traverse_result["start_room"], "shared-room");
    assert!(traverse_result["connections"]
        .as_array()
        .expect("connections array")
        .is_empty());

    let tunnels_args = tools::graph::FindTunnelsArgs {
        wing_a: Some("wing_alpha".to_string()),
        wing_b: Some("wing_beta".to_string()),
    };
    let tunnels_result = parse_json(
        tools::graph::find_tunnels(&harness.server, &tunnels_args)
            .await
            .expect("find_tunnels should succeed"),
    );
    assert_eq!(
        tunnels_result["tunnels"]
            .as_array()
            .expect("tunnels array")
            .len(),
        1
    );
    assert_eq!(tunnels_result["tunnels"][0]["room"], "shared-room");
}

#[tokio::test]
async fn knowledge_graph_tools_return_seeded_data() {
    let harness = McpTestHarness::new(true);
    harness
        .add_triple(
            "triple-1",
            "memory",
            "relates_to",
            "palace",
            Some(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date")),
            None,
        )
        .await;
    harness
        .add_triple(
            "triple-2",
            "memory",
            "depends_on",
            "context",
            Some(chrono::NaiveDate::from_ymd_opt(2024, 2, 1).expect("valid date")),
            None,
        )
        .await;

    let query = parse_json(
        tools::knowledge_graph::kg_query(
            &harness.server,
            &tools::knowledge_graph::KgQueryArgs {
                entity: "memory".to_string(),
                as_of: Some("2024-03-01".to_string()),
                direction: Some("outgoing".to_string()),
            },
        )
        .now_or_never()
        .expect("kg_query should complete immediately")
        .expect("kg_query should succeed"),
    );
    assert_eq!(query["count"], 2);
    assert_eq!(query["entity"], "memory");

    let timeline = parse_json(
        tools::knowledge_graph::kg_timeline(
            &harness.server,
            &tools::knowledge_graph::KgTimelineArgs {
                entity: Some("memory".to_string()),
            },
        )
        .now_or_never()
        .expect("kg_timeline should complete immediately")
        .expect("kg_timeline should succeed"),
    );
    assert_eq!(timeline["count"], 2);

    let stats = parse_json(
        tools::knowledge_graph::kg_stats(&harness.server)
            .await
            .expect("kg_stats should succeed"),
    );
    assert_eq!(stats["triples"], 2);
    assert_eq!(stats["entities"], 0);
}

#[tokio::test]
async fn diary_read_returns_recent_entries_for_agent() {
    let harness = McpTestHarness::new(false);
    harness
        .add_drawer(
            "diary-1",
            "AAAK entry one",
            "wing_test_agent",
            "diary/general",
            "diary",
            "test agent",
            IngestMode::Convos,
        )
        .await;
    harness
        .add_drawer(
            "diary-2",
            "AAAK entry two",
            "wing_test_agent",
            "diary/general",
            "diary",
            "test agent",
            IngestMode::Convos,
        )
        .await;
    harness
        .add_drawer(
            "other-1",
            "Other agent entry",
            "wing_other_agent",
            "diary/general",
            "diary",
            "other agent",
            IngestMode::Convos,
        )
        .await;

    let diary = parse_json(
        tools::diary::diary_read(
            &harness.server,
            &tools::diary::DiaryReadArgs {
                agent_name: "Test Agent".to_string(),
                last_n: Some(5),
            },
        )
        .now_or_never()
        .expect("diary_read should complete immediately")
        .expect("diary_read should succeed"),
    );
    assert_eq!(diary["agent"], "Test Agent");
    assert_eq!(diary["total"], 2);
    assert_eq!(diary["entries"].as_array().expect("entries array").len(), 2);
}
