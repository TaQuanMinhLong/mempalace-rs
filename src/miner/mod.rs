//! Mining modules - file ingestion, conversation mining, file splitting

pub mod convo_miner;
pub mod file_miner;
pub mod splitter;

pub use convo_miner::ConvoMiner;
pub use file_miner::FileMiner;
pub use splitter::MegaFileSplitter;
