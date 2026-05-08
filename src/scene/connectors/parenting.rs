use super::super::{NodeKey, Scene};

pub(super) fn node_is_descendant_of(scene: &Scene, candidate: NodeKey, ancestor: NodeKey) -> bool {
    let mut current = Some(candidate);
    while let Some(node) = current {
        if node == ancestor {
            return true;
        }
        current = scene.nodes.get(node).and_then(|node| node.parent());
    }
    false
}
