//! Search modules - semantic search and retrieval

pub mod retrieval;
pub mod semantic;

pub use retrieval::{
    RetrievalExplanation, RetrievalMode, RetrievalPlan, RetrievalResult, RetrieveOptions, Retriever,
};
pub use semantic::SemanticSearcher;
