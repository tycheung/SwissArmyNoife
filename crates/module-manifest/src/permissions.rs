//! Permission declarations → policy defaults (`sak356`).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Declared capability the module may request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDecl {
    /// e.g. `network.egress`, `fs.read`, `llm.chat`
    pub name: String,
    #[serde(default)]
    pub optional: bool,
}

/// Map permission names into a starter policy JSON fragment.
#[must_use]
pub fn permissions_to_policy_defaults(perms: &[PermissionDecl]) -> Value {
    let mut allow_tools = Vec::new();
    let mut egress = false;
    for p in perms {
        match p.name.as_str() {
            "network.egress" | "egress" => egress = true,
            other => allow_tools.push(other.to_owned()),
        }
    }
    let mut policy = json!({});
    if !allow_tools.is_empty() {
        policy["tools"] = json!({ "allow": allow_tools });
    }
    if egress {
        policy["egress"] = json!({
            "allow_hosts": [],
            "allow_principals": ["local"],
            "max_response_bytes": 65536
        });
    }
    policy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_egress_and_tools() {
        let perms = vec![
            PermissionDecl {
                name: "network.egress".into(),
                optional: false,
            },
            PermissionDecl {
                name: "fs.read".into(),
                optional: true,
            },
        ];
        let p = permissions_to_policy_defaults(&perms);
        assert!(p["egress"]["max_response_bytes"].as_u64().is_some());
        assert_eq!(p["tools"]["allow"][0], "fs.read");
    }
}
