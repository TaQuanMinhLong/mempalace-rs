use crate::error::Result;

pub async fn run() -> Result<()> {
    tracing::info!("starting mcp stdio server");
    if let Err(error) = crate::mcp::serve().await {
        tracing::error!("mcp stdio server exited with error: {}", error);
        return Err(error);
    }
    tracing::info!("mcp stdio server shut down cleanly");
    Ok(())
}
