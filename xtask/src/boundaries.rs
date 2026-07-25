//! Crate dependency boundary checks (see `.cursor/rules/crate-boundaries.mdc`).

use std::fs;
use std::path::{Path, PathBuf};

/// A directed path-dependency edge `from` → `to` (package names).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

/// Why an edge is forbidden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub edge: Edge,
    pub reason: &'static str,
}

/// Layer in the allowed DAG (lower may not depend on higher).
fn layer(name: &str) -> Option<u8> {
    if name == "xtask" {
        return None;
    }
    if name.starts_with("provider-") {
        return Some(0);
    }
    if name.starts_with("offer-")
        || name == "module-manifest"
        || name == "module-registry"
        || name.starts_with("runtime-")
    {
        return Some(1);
    }
    match name {
        "types" | "env" | "vault" => Some(0),
        "persist-sqlite" | "control" => Some(1),
        "cli" | "mcp" | "http" | "http-admin" | "compute-worker" | "sdk" => Some(2),
        _ => None,
    }
}

/// Return a reason if `from` → `to` is forbidden.
#[must_use]
pub fn forbidden_reason(from: &str, to: &str) -> Option<&'static str> {
    let to_l = to.to_ascii_lowercase();
    if to_l.contains("nimbusware") {
        return Some("dependency on Nimbusware is forbidden");
    }
    if from.starts_with("offer-") {
        if to == "mcp" || to == "http" || to == "http-admin" || to.starts_with("mcp") {
            return Some("offer crates must not depend on mcp/http adapters");
        }
        if to.starts_with("offer-") && from != to {
            return Some("cross-offer dependencies are forbidden");
        }
    }
    if from.starts_with("provider-")
        && (to == "control" || to == "mcp" || to == "http" || to == "http-admin" || to == "cli")
    {
        return Some("provider crates must not depend on control-plane/adapters");
    }
    match (layer(from), layer(to)) {
        (Some(from_layer), Some(to_layer)) if from_layer < to_layer => {
            Some("upward dependency (lower layer must not depend on higher layer)")
        }
        _ => None,
    }
}

/// Check a list of edges; return all violations.
#[must_use]
pub fn check_edges(edges: &[Edge]) -> Vec<Violation> {
    edges
        .iter()
        .filter_map(|edge| {
            forbidden_reason(&edge.from, &edge.to).map(|reason| Violation {
                edge: edge.clone(),
                reason,
            })
        })
        .collect()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask parent")
        .to_path_buf()
}

fn package_name(cargo_toml: &Path) -> Option<String> {
    let text = fs::read_to_string(cargo_toml).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_owned)
}

fn path_dep_names(cargo_toml: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(cargo_toml) else {
        return Vec::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for table_key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(deps) = value.get(table_key).and_then(|v| v.as_table()) else {
            continue;
        };
        for (name, spec) in deps {
            if spec.get("path").is_some() {
                let pkg = spec.get("package").and_then(|v| v.as_str()).unwrap_or(name);
                out.push(pkg.to_owned());
            }
        }
    }
    out
}

fn collect_workspace_edges(root: &Path) -> Vec<Edge> {
    let mut edges = Vec::new();
    let crates_dir = root.join("crates");
    let Ok(entries) = fs::read_dir(&crates_dir) else {
        return edges;
    };
    for entry in entries.flatten() {
        let cargo = entry.path().join("Cargo.toml");
        if !cargo.is_file() {
            continue;
        }
        let Some(from) = package_name(&cargo) else {
            continue;
        };
        for to in path_dep_names(&cargo) {
            edges.push(Edge {
                from: from.clone(),
                to,
            });
        }
    }
    edges
}

/// Scan the workspace and return violations (empty = OK).
pub fn check_workspace() -> Result<(), Vec<String>> {
    let root = workspace_root();
    let edges = collect_workspace_edges(&root);
    let violations = check_edges(&edges);
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations
            .into_iter()
            .map(|v| format!("{} → {}: {}", v.edge.from, v.edge.to, v.reason))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_cli_to_types() {
        let edges = [Edge {
            from: "cli".into(),
            to: "types".into(),
        }];
        assert!(check_edges(&edges).is_empty());
    }

    #[test]
    fn rejects_types_depending_on_control() {
        let edges = [Edge {
            from: "types".into(),
            to: "control".into(),
        }];
        let v = check_edges(&edges);
        assert_eq!(v.len(), 1);
        assert!(v[0].reason.contains("upward"));
    }

    #[test]
    fn rejects_nimbusware_edge() {
        let edges = [Edge {
            from: "control".into(),
            to: "nimbusware_agent_tools".into(),
        }];
        let v = check_edges(&edges);
        assert_eq!(v.len(), 1);
        assert!(v[0].reason.contains("Nimbusware"));
    }

    #[test]
    fn rejects_cross_offer() {
        let edges = [Edge {
            from: "offer-memory".into(),
            to: "offer-sandbox".into(),
        }];
        let v = check_edges(&edges);
        assert_eq!(v.len(), 1);
        assert!(v[0].reason.contains("cross-offer"));
    }

    #[test]
    fn workspace_currently_clean() {
        check_workspace().expect("workspace edges should be clean");
    }
}
