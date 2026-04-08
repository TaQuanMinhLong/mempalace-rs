//! MCP server - Model Context Protocol server using official rust-sdk
//!
//! Migrated from manual JSON-RPC implementation to rmcp crate

use crate::config::Config;
use crate::mcp::tools;
use crate::mcp::tools::catalog::ListRoomsArgs;
use crate::mcp::tools::diary::{DiaryReadArgs, DiaryWriteArgs};
use crate::mcp::tools::drawers::{AddDrawerArgs, DeleteDrawerArgs};
use crate::mcp::tools::graph::{FindTunnelsArgs, TraverseArgs};
use crate::mcp::tools::knowledge_graph::{
    KgAddArgs, KgInvalidateArgs, KgQueryArgs, KgTimelineArgs,
};
use crate::mcp::tools::protocol::PALACE_PROTOCOL;
use crate::mcp::tools::search::{CheckDuplicateArgs, SearchArgs};
use crate::storage::{ChromaStorage, KnowledgeGraph};
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct McpServer {
    pub config: Config,
    pub storage: Arc<Mutex<ChromaStorage>>,
    pub knowledge_graph: Option<Arc<Mutex<KnowledgeGraph>>>,
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    #[cfg(test)]
    pub(crate) fn mock(
        config: Config,
        storage: ChromaStorage,
        knowledge_graph: Option<KnowledgeGraph>,
    ) -> Self {
        Self {
            config,
            storage: Arc::new(Mutex::new(storage)),
            knowledge_graph: knowledge_graph.map(|kg| Arc::new(Mutex::new(kg))),
            tool_router: Self::tool_router(),
        }
    }

    pub fn try_new() -> crate::error::Result<Self> {
        let config = Config::load()?;
        let storage = ChromaStorage::new(&config.palace_path, &config.collection_name)?;
        let knowledge_graph = if config.knowledge_graph_path.exists() {
            KnowledgeGraph::new(&config.knowledge_graph_path)
                .ok()
                .map(|kg| Arc::new(Mutex::new(kg)))
        } else {
            None
        };
        Ok(Self {
            config,
            storage: Arc::new(Mutex::new(storage)),
            knowledge_graph,
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl McpServer {
    #[tool(description = "Palace overview — total drawers, wing and room counts")]
    async fn status(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::catalog::status(self).await
    }

    #[tool(description = "List all wings with drawer counts")]
    async fn list_wings(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::catalog::list_wings(self).await
    }

    #[tool(description = "List rooms within a wing (or all rooms if no wing given)")]
    async fn list_rooms(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<ListRoomsArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::catalog::list_rooms(self, &params.0).await
    }

    #[tool(description = "Full taxonomy: wing → room → drawer count")]
    async fn get_taxonomy(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::catalog::get_taxonomy(self).await
    }

    #[tool(description = "Get the AAAK dialect specification")]
    fn get_aaak_spec(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::catalog::get_aaak_spec()
    }

    #[tool(
        description = "Semantic search. Returns verbatim drawer content with similarity scores."
    )]
    async fn search(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<SearchArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::search::search(self, &params.0).await
    }

    #[tool(description = "Check if content already exists in the palace before filing")]
    async fn check_duplicate(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<CheckDuplicateArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::search::check_duplicate(self, &params.0).await
    }

    #[tool(description = "File verbatim content into the palace. Checks for duplicates first.")]
    async fn add_drawer(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<AddDrawerArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::drawers::add_drawer(self, &params.0).await
    }

    #[tool(description = "Delete a drawer by ID. Irreversible.")]
    async fn delete_drawer(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<DeleteDrawerArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::drawers::delete_drawer(self, &params.0).await
    }

    #[tool(description = "Query the knowledge graph for an entity's relationships")]
    async fn kg_query(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<KgQueryArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::knowledge_graph::kg_query(self, &params.0).await
    }

    #[tool(description = "Add a fact to the knowledge graph")]
    async fn kg_add(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<KgAddArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::knowledge_graph::kg_add(self, &params.0).await
    }

    #[tool(description = "Mark a fact as no longer true")]
    async fn kg_invalidate(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<KgInvalidateArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::knowledge_graph::kg_invalidate(self, &params.0).await
    }

    #[tool(description = "Chronological timeline of facts")]
    async fn kg_timeline(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<KgTimelineArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::knowledge_graph::kg_timeline(self, &params.0).await
    }

    #[tool(description = "Knowledge graph overview")]
    async fn kg_stats(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::knowledge_graph::kg_stats(self).await
    }

    #[tool(description = "Walk the palace graph from a room")]
    async fn traverse(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<TraverseArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::graph::traverse(self, &params.0).await
    }

    #[tool(description = "Find rooms that bridge two wings")]
    async fn find_tunnels(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<FindTunnelsArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::graph::find_tunnels(self, &params.0).await
    }

    #[tool(description = "Palace graph overview")]
    async fn graph_stats(&self) -> std::result::Result<CallToolResult, McpError> {
        tools::graph::graph_stats(self).await
    }

    #[tool(description = "Write to your personal agent diary")]
    async fn diary_write(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<DiaryWriteArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::diary::diary_write(self, &params.0).await
    }

    #[tool(description = "Read your recent diary entries")]
    async fn diary_read(
        &self,
        params: rmcp::handler::server::wrapper::Parameters<DiaryReadArgs>,
    ) -> std::result::Result<CallToolResult, McpError> {
        tools::diary::diary_read(self, &params.0).await
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "mempalace",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(PALACE_PROTOCOL.to_string())
    }
}

pub async fn serve() -> crate::error::Result<()> {
    let server = McpServer::try_new()?;
    let service = server
        .serve(stdio())
        .await
        .map_err(|error| crate::error::MempalaceError::Config(error.to_string()))?;
    service
        .waiting()
        .await
        .map_err(|error| crate::error::MempalaceError::Config(error.to_string()))?;
    Ok(())
}
