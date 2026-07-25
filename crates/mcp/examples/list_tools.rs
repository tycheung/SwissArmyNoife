use rmcp::{
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bin = r"D:\\Agentic\\SwissArmyNoife\\target\\debug\\mcp.exe";
    let tmp = tempfile::tempdir()?;
    let client = ()
        .serve(TokioChildProcess::new(Command::new(bin).configure(|c| {
            c.env("CONFIG_DIR", tmp.path())
                .env("LLM_BACKEND", "echo")
                .env("SANDBOX_BACKEND", "none");
        }))?)
        .await?;
    let listed = client.list_tools(Option::default()).await?;
    println!("count={}", listed.tools.len());
    for t in &listed.tools {
        let schema = serde_json::to_string(&t.input_schema).unwrap_or_default();
        println!("{} schema_bytes={}", t.name, schema.len());
    }
    if let Some(next) = listed.next_cursor.as_ref() {
        println!("NEXT_CURSOR={next}");
    } else {
        println!("NEXT_CURSOR=none");
    }
    client.cancel().await?;
    Ok(())
}
