//! Minimal worker (`sak293`): register → claim → complete.
//!
//! `COMPUTE_QUEUE=sqlite` (and `CONFIG_DIR`/`DB_PATH`) shares durable queue with MCP/HTTP.

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use offer_compute::{ComputePlane, NodeId};
use serde_json::json;
use types::ErrorCode;

fn main() -> ExitCode {
    let label = env::var("WORKER_LABEL").unwrap_or_else(|_| "compute-worker".into());
    let plane = match boot_plane() {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("boot plane failed: {e}");
            return ExitCode::from(1);
        }
    };

    let demo = env::var("WORKER_DEMO").unwrap_or_else(|_| "1".into()) == "1";

    let node = match plane.nodes.register(&label, vec!["echo".into()], None) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("register failed: {e}");
            return ExitCode::from(1);
        }
    };
    println!("registered {} queue={}", node.id, queue_label());

    if demo {
        match plane.queue.enqueue("echo", json!({ "hello": "world" })) {
            Ok(u) => println!("enqueued {}", u.id),
            Err(e) => {
                eprintln!("enqueue failed: {e}");
                return ExitCode::from(1);
            }
        }
    }

    if let Err(code) = run_once(&plane, node.id) {
        eprintln!("worker failed: {code}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn boot_plane() -> Result<ComputePlane, ErrorCode> {
    ComputePlane::from_env()
}

fn queue_label() -> &'static str {
    let mode = env::var("COMPUTE_QUEUE").unwrap_or_else(|_| "sqlite".into());
    if mode.eq_ignore_ascii_case("memory") {
        "memory"
    } else if mode.eq_ignore_ascii_case("redis") {
        "redis"
    } else {
        "sqlite"
    }
}

fn run_once(plane: &ComputePlane, node: NodeId) -> Result<(), ErrorCode> {
    let claimed = plane.queue.claim(node)?;
    let result = json!({
        "echo": claimed.payload,
        "worker": node.to_string(),
    });
    let done = plane
        .queue
        .complete(claimed.id, node, result, plane.merge.as_ref())?;
    println!("completed {} status={}", done.id, done.status.as_str());
    Ok(())
}
