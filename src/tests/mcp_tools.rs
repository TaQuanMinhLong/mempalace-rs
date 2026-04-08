use crate::config::Config;
use crate::mcp::server::McpServer;
use crate::palace::{Drawer, DrawerMetadata, IngestMode};
use crate::storage::{ChromaStorage, KnowledgeGraph, Triple};
use chrono::{NaiveDate, Utc};
use tempfile::TempDir;

pub(crate) struct McpTestHarness {
    _temp_dir: TempDir,
    pub server: McpServer,
}

impl McpTestHarness {
    pub(crate) fn new(with_knowledge_graph: bool) -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir should create successfully");
        let palace_path = temp_dir.path().join("palace");
        let knowledge_graph_path = temp_dir.path().join("knowledge_graph.sqlite3");

        let storage = ChromaStorage::new(&palace_path, "test_collection")
            .expect("temporary search storage should initialize");

        let knowledge_graph = if with_knowledge_graph {
            Some(
                KnowledgeGraph::new(&knowledge_graph_path)
                    .expect("temporary knowledge graph should initialize"),
            )
        } else {
            None
        };

        let config = Config {
            palace_path,
            collection_name: "test_collection".to_string(),
            knowledge_graph_path,
            identity_path: temp_dir.path().join("identity.txt"),
            config_dir: temp_dir.path().to_path_buf(),
            ..Config::default()
        };

        let server = McpServer::mock(config, storage, knowledge_graph);

        Self {
            _temp_dir: temp_dir,
            server,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn add_drawer(
        &self,
        id: &str,
        text: &str,
        wing: &str,
        room: &str,
        source_file: &str,
        added_by: &str,
        ingest_mode: IngestMode,
    ) {
        let metadata = DrawerMetadata::new(wing, room, source_file, 0, added_by, ingest_mode);
        let drawer = Drawer::new(id, text, metadata);
        self.server
            .storage
            .lock()
            .await
            .add_drawer(&drawer)
            .expect("seed drawer should be stored");
    }

    pub(crate) async fn add_triple(
        &self,
        id: &str,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Option<NaiveDate>,
        valid_to: Option<NaiveDate>,
    ) {
        let kg = self
            .server
            .knowledge_graph
            .as_ref()
            .expect("knowledge graph should be enabled for this harness");

        let triple = Triple {
            id: id.to_string(),
            subject: subject.to_string(),
            predicate: predicate.to_string(),
            object: object.to_string(),
            valid_from,
            valid_to,
            confidence: 1.0,
            source_closet: "test_closet".to_string(),
            source_file: "test_source".to_string(),
            extracted_at: Utc::now(),
        };

        kg.lock()
            .await
            .upsert_triple(&triple)
            .expect("seed triple should be stored");
    }
}
