use std::collections::VecDeque;

use super::*;

/// Sizes of the tables a node may index into.
#[derive(Clone, Copy)]
pub(super) struct NodeReferenceCounts {
    pub(super) nodes: usize,
    pub(super) meshes: usize,
    pub(super) skins: usize,
    pub(super) cameras: usize,
}

/// Validates node references and proves the node graph is a forest within
/// the import limits. Later stages walk the hierarchy with explicit work
/// lists, and the depth and count limits bound how much work a document can
/// demand from them.
pub(super) fn validate_node_graph(
    path: &AssetPath,
    value: Option<&Value>,
    counts: NodeReferenceCounts,
    limits: &GltfImportLimits,
) -> Result<(), AssetError> {
    let Some(nodes) = value.and_then(Value::as_array) else {
        return Ok(());
    };
    let node_count = counts.nodes;
    if node_count > limits.max_node_count {
        return Err(AssetError::PolicyViolation {
            path: path.as_str().to_string(),
            reason: format!(
                "the document declares {node_count} nodes, exceeding max_node_count {}",
                limits.max_node_count
            ),
            help: "split the asset or raise the limit with AssetLoadOptions::with_max_node_count",
        });
    }
    let mut indegree = vec![0_usize; node_count];
    let mut edges = vec![Vec::new(); node_count];
    for (node_index, node) in nodes.iter().enumerate() {
        let Some(node) = node.as_object() else {
            continue;
        };
        validate_index(
            path,
            node.get("mesh"),
            &format!("$.nodes[{node_index}].mesh"),
            counts.meshes,
        )?;
        validate_index(
            path,
            node.get("skin"),
            &format!("$.nodes[{node_index}].skin"),
            counts.skins,
        )?;
        validate_index(
            path,
            node.get("camera"),
            &format!("$.nodes[{node_index}].camera"),
            counts.cameras,
        )?;
        let Some(children) = node.get("children").and_then(Value::as_array) else {
            continue;
        };
        for (child_offset, child) in children.iter().enumerate() {
            let child_path = format!("$.nodes[{node_index}].children[{child_offset}]");
            let child = required_index(path, child, &child_path, node_count)?;
            indegree[child] += 1;
            if indegree[child] > 1 {
                return Err(parse_error(
                    path,
                    format!(
                        "{child_path} gives node {child} multiple parents; glTF nodes must form a tree/forest, not a DAG"
                    ),
                ));
            }
            edges[node_index].push(child);
        }
    }

    // Kahn's algorithm visits every parent before its children, so one pass
    // both detects cycles and yields each node's depth; a root has depth 1.
    let mut ready = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut depth = vec![1_usize; node_count];
    let mut deepest = None::<(usize, usize)>;
    let mut visited = 0_usize;
    while let Some(node) = ready.pop_front() {
        visited += 1;
        if deepest.is_none_or(|(deepest_depth, _)| depth[node] > deepest_depth) {
            deepest = Some((depth[node], node));
        }
        for child in &edges[node] {
            depth[*child] = depth[node] + 1;
            indegree[*child] -= 1;
            if indegree[*child] == 0 {
                ready.push_back(*child);
            }
        }
    }
    if visited != node_count {
        return Err(parse_error(
            path,
            "$.nodes contains a cycle; glTF node graphs must be acyclic",
        ));
    }
    if let Some((deepest_depth, deepest_node)) = deepest
        && deepest_depth > limits.max_node_depth
    {
        return Err(AssetError::PolicyViolation {
            path: path.as_str().to_string(),
            reason: format!(
                "node {deepest_node} is nested {deepest_depth} levels deep, exceeding max_node_depth {}",
                limits.max_node_depth
            ),
            help: "flatten the node hierarchy or raise the limit with AssetLoadOptions::with_max_node_depth",
        });
    }
    Ok(())
}
