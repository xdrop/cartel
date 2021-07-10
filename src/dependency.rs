use crate::collections::{FromIndexContainer, FromOwnedIndexContainer, VecExt};
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

pub struct DependencyGraph<'a, T, M>
where
    T: WithDependencies<M> + Eq + Hash,
    M: PartialOrd + Default,
{
    edge_map: HashMap<String, Vec<DependencyNode<&'a T, M>>>,
    node_list: Vec<DependencyNode<&'a T, M>>,
}

#[derive(Debug)]
pub struct DependencyNode<T, M> {
    /// The unique identifier for this dependency node.
    pub key: String,
    /// The original value this dependency node wraps.
    pub value: T,
    /// The marker for the node (if it has been marked by an edge). None otherwise.
    pub marker: Option<M>,
    /// The 'origin' nodes that caused this dependency to be included in the
    /// dependency graph.
    pub origin_nodes: HashSet<String>,
    /// Whether this node is "weak". A is_weak node should only be included in the
    /// final graph if a dependency of at least one strong node.
    pub is_weak: bool,
}

pub enum EdgeDirection {
    /// The edge points to the given node.
    To,
    /// The originates from the given node.
    From,
}

pub struct DependencyEdge<M: PartialOrd> {
    /// The key of the node this edge originates from.
    pub edge_src: String,
    /// The key of the node this edge points to.
    pub edge_dst: String,
    /// The direction this edge points to.
    pub direction: EdgeDirection,
    /// A marker is used to mark the node that this edge points to. Since
    /// multiple edges may be pointing to the same node with potentially
    /// different markers, the marker type must implement [PartialOrd].
    pub marker: M,
    /// Whether this edge is "weak". A weak edge would not cause the target node
    /// to appear in the graph, but if the target node is already present then a
    /// link is enforced between them.
    ///
    /// This is used to constrain a node to be always ordered after another, but
    /// not become a dependency in the graph unless it is also a direct
    /// dependency of some other node.
    pub is_weak: bool,
}

pub trait WithDependencies<M: PartialOrd>: WithKey {
    fn dependencies(&self) -> Vec<DependencyEdge<M>>;
    fn is_group(&self) -> bool;
}

pub trait WithKey {
    fn key(&self) -> String;
    fn key_ref(&self) -> &str;
}

impl<T: Copy, M: Copy> Clone for DependencyNode<T, M> {
    fn clone(&self) -> Self {
        DependencyNode {
            key: self.key.clone(),
            value: self.value,
            is_weak: self.is_weak,
            marker: self.marker,
            origin_nodes: self.origin_nodes.clone(),
        }
    }
}

impl<T, M> Hash for DependencyNode<T, M> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.key.hash(state);
    }
}

impl<T, M> PartialEq for DependencyNode<T, M> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T, M> Eq for DependencyNode<T, M> {}

enum MarkType {
    PERMANENT,
    TEMPORARY,
}

#[derive(Debug)]
struct StackEntry {
    // The index of the node in the arena
    node_idx: usize,
    // The index of the origin node that this node originated from
    origin_node_idx: usize,
}

impl StackEntry {
    pub fn new(idx: usize) -> Self {
        Self {
            node_idx: idx,
            origin_node_idx: idx,
        }
    }

    pub fn new_related(idx: usize, origin_idx: usize) -> Self {
        Self {
            node_idx: idx,
            origin_node_idx: origin_idx,
        }
    }
}

/// A dependency graph over type T.
impl<'a, T, M> DependencyGraph<'a, T, M>
where
    T: WithDependencies<M> + Eq + Hash,
    M: PartialOrd + Copy + Default,
{
    /// Build a partial dependency graph from a list of module definitions.
    ///
    /// The dependency graph is built through the declared dependencies of each
    /// module (the `dependencies` field on the struct). Only dependencies for
    /// modules included in `selected` are considered
    ///
    /// # Arguments:
    /// * `mod_defs` - The list of module definitions to use
    /// * `selected` - The modules (and their dependencies) to be included.
    ///
    /// # Examples:
    /// ```ignore
    /// let graph = DepedencyGraph::from(mod_defs, &vec!["a", "b"]);
    /// let sorted = graph.dependency_sort();
    /// ```
    pub fn from(
        src: &'a [T],
        selected: &'a [&str],
    ) -> DependencyGraph<'a, T, M> {
        let (mut arena, selection_indices) =
            GraphArena::<T, M>::populate(src, selected);

        let mut node_stack: Vec<StackEntry> = selection_indices
            .iter()
            .map(|idx| StackEntry::new(*idx))
            .collect();

        while !node_stack.is_empty() {
            let node_stack_entry = node_stack.pop().unwrap();
            let node_idx = node_stack_entry.node_idx;
            let mut origin_node_idx = node_stack_entry.origin_node_idx;

            let node = arena.get_by_idx(node_idx);
            let dependencies = node.value.dependencies();

            dependencies.iter().for_each(|edge| {
                let edge_dst = &edge.edge_dst;
                let edge_src = &edge.edge_src;
                let marker = edge.marker;
                let original_node =
                    arena.get_original_node(edge_dst.as_str(), src);

                // Ensure the node pointed to exists, otherwise create it.
                let (pointed_to_idx, was_created) = arena.get_or_create(
                    edge_dst,
                    original_node,
                    origin_node_idx,
                    edge.is_weak,
                    marker,
                );

                // If we are dealing with a group we want to mark this node as
                // the parent.
                if arena.get_by_idx(origin_node_idx).value.is_group() {
                    origin_node_idx = node_idx;
                }

                if was_created && !edge.is_weak {
                    // Push it to the stack so we visit its dependencies next
                    // (unless it is a weak node in which case we want to skip)
                    node_stack.push(StackEntry::new_related(
                        pointed_to_idx,
                        origin_node_idx,
                    ));
                }

                let (key, idx) = match edge.direction {
                    EdgeDirection::To => (edge_src.as_str(), pointed_to_idx),
                    EdgeDirection::From => {
                        arena.push_node_idx(pointed_to_idx);
                        (edge_dst.as_str(), node_idx)
                    }
                };

                Self::maybe_upgrade_marker(marker, arena.get_mut_ref(idx));
                arena.add_edge(key, idx);
            })
        }

        let (edge_map, node_list) = arena.dispose();
        DependencyGraph {
            edge_map,
            node_list,
        }
    }

    /// Upgrade the current marker on the dependency node.
    ///
    /// Checks the existing marker on the node, if the new marker is higher
    /// ranked than the current one, then the node is marked with the new marker
    /// instead.
    ///
    /// If no marker existed on the node, then the node is marked with the new
    /// marker instead.
    fn maybe_upgrade_marker(new_marker: M, node: &mut DependencyNode<&T, M>) {
        match node.marker {
            Some(marker) => {
                if new_marker > marker {
                    node.marker = Some(new_marker);
                }
            }
            None => {
                node.marker = Some(new_marker);
            }
        }
    }

    /// Split nodes into groups by their level.
    fn split_by_level<'l, 's, R: Eq + Hash>(
        level_info: HashMap<&'l R, NodeMeta>,
        sorted_nodes: Vec<&'s R>,
    ) -> SortedDeps<'s, R> {
        let mut groups = Vec::new();
        for node in sorted_nodes.iter().rev() {
            let level = level_info.get(node).unwrap().level;
            if level >= groups.len() as u8 {
                groups.push(Vec::new());
            }
            groups[level as usize].push(*node);
        }

        groups.reverse();

        SortedDeps {
            groups,
            flat: sorted_nodes,
        }
    }

    /// Return a sorted list of dependencies.
    ///
    /// Sorts dependencies so that dependent modules are deployed before the
    /// modules that depend on them. The topological sort is performed using
    /// modified DFS.
    pub fn dependency_sort(&self) -> Result<Vec<&DependencyNode<&T, M>>> {
        let mut sorted = Vec::new();
        let mut stack: Vec<(bool, &DependencyNode<&T, M>)> = Vec::new();
        let mut marked: HashMap<_, MarkType> = HashMap::new();
        let mut unmarked: Vec<_> = self.node_list.iter().collect();

        // While we have still nodes unmarked
        while !unmarked.is_empty() {
            let to_mark = unmarked.pop().unwrap();
            stack.push((false, to_mark));

            while !stack.is_empty() {
                let (is_parent, node) = stack.pop().unwrap();

                if node.is_weak {
                    // Weak nodes are not to be included in the final output.
                    continue;
                }

                if is_parent {
                    sorted.push(node);
                    marked.entry(node).and_modify(|e| *e = MarkType::PERMANENT);
                    continue;
                }

                if let Some(mark) = marked.get(node) {
                    match mark {
                        MarkType::PERMANENT => continue,
                        MarkType::TEMPORARY => {
                            bail!("The graph contains cycles")
                        }
                    }
                }

                marked.insert(node, MarkType::TEMPORARY);
                stack.push((true, node));

                for edge in self.edge_map.get(&node.key).unwrap() {
                    stack.push((false, edge));
                }
            }
        }
        Ok(sorted)
    }

    /// Sort dependencies and return in them in groups.
    ///
    /// Sorts dependencies into groups so that dependent modules are deployed
    /// before the modules that depend on them. Each group represents a set of
    /// dependencies that have no ordering between them. The topological sort is
    /// performed using modified DFS.
    pub fn group_sort(&self) -> Result<SortedDeps<DependencyNode<&T, M>>> {
        let mut sorted = Vec::new();
        let mut stack: Vec<(bool, &DependencyNode<&T, M>, u8)> = Vec::new();
        let mut marked: HashMap<_, NodeMeta> = HashMap::new();
        let mut unmarked: Vec<_> = self.node_list.iter().collect();

        // While we have still nodes unmarked
        while !unmarked.is_empty() {
            let to_mark = unmarked.pop().unwrap();
            stack.push((false, to_mark, 0));

            while !stack.is_empty() {
                let (is_parent, node, level) = stack.pop().unwrap();

                if node.is_weak {
                    // Weak nodes are not to be included in the final output.
                    continue;
                }

                let edges = self.edge_map.get(&node.key).unwrap();

                if is_parent {
                    sorted.push(node);
                    marked.entry(node).and_modify(|e| {
                        e.mark = MarkType::PERMANENT;
                        e.level = level;
                    });
                    continue;
                }

                if let Some(meta) = marked.get_mut(node) {
                    match meta.mark {
                        MarkType::PERMANENT => {
                            if level > meta.level {
                                meta.level = level;

                                // Propagate new level to connected edges
                                for edge in edges {
                                    stack.push((false, edge, level + 1));
                                }
                            }
                            continue;
                        }
                        MarkType::TEMPORARY => {
                            bail!("The graph contains cycles")
                        }
                    }
                }

                marked.insert(node, meta(MarkType::TEMPORARY, level));
                stack.push((true, node, level));

                for edge in edges {
                    stack.push((false, edge, level + 1));
                }
            }
        }

        // Sort into groups based on their level.
        Ok(Self::split_by_level(marked, sorted))
    }
}

pub struct SortedDeps<'a, R> {
    pub groups: Vec<Vec<&'a R>>,
    pub flat: Vec<&'a R>,
}

/// Contains information about each node used in the topological sort.
struct NodeMeta {
    mark: MarkType,
    level: u8,
}

fn meta(mark: MarkType, level: u8) -> NodeMeta {
    NodeMeta { mark, level }
}

// Disposable data structure used while building a graph. Holds all items
// in an arena.
struct GraphArena<'a, S, M> {
    /// Holds the edges of each node (their indices).
    edge_map: HashMap<String, Vec<usize>>,
    /// Holds the index of each node in the arena.
    node_map: HashMap<String, usize>,
    /// Holds a list of all nodes that should be exposed out of the arena.
    node_list: Vec<usize>,
    /// Holds the index of each node in the *source* array (not the arena).
    source_array_index: HashMap<&'a str, usize>,
    /// Arena holding all dependency nodes.
    arena: Vec<DependencyNode<&'a S, M>>,
}

impl<'a, S, M> GraphArena<'a, S, M>
where
    S: WithDependencies<M> + Eq + Hash,
    M: PartialOrd + Copy + Default,
{
    /// Get an arena node by index.
    pub fn get_by_idx(&self, idx: usize) -> &DependencyNode<&S, M> {
        &self.arena[idx]
    }

    /// Includes a node in the node list.
    pub fn push_node_idx(&mut self, idx: usize) {
        self.node_list.push(idx);
    }

    /// Gets the original node out of the souce array.
    pub fn get_original_node(&self, key: &str, source: &'a [S]) -> &'a S {
        &source[self.source_array_index[key]]
    }

    /// Adds a new edge to the node with the given key.
    pub fn add_edge(&mut self, src_key: &str, idx: usize) {
        self.edge_map.get_mut(src_key).unwrap().push(idx);
    }

    /// Get a mutable ref for the node at the given index.
    pub fn get_mut_ref(&mut self, idx: usize) -> &mut DependencyNode<&'a S, M> {
        &mut self.arena[idx]
    }

    /// Get or create a node in the arena.
    pub fn get_or_create(
        &mut self,
        key: &str,
        original_node: &'a S,
        origin_idx: usize,
        is_weak: bool,
        marker: M,
    ) -> (usize, bool) {
        let origin_key = self.get_by_idx(origin_idx).key.clone();
        match self.node_map.get(key).copied() {
            Some(idx) => {
                let existing = self.get_mut_ref(idx);
                // If this node was weak but got referenced by a non-weak edge,
                // upgrade this node to strong.
                if existing.is_weak && !is_weak {
                    existing.is_weak = false;
                };
                existing.origin_nodes.insert(origin_key);
                (idx, false)
            }
            None => {
                let new_node = Self::new_node(
                    key.to_string(),
                    original_node,
                    is_weak,
                    Some(marker),
                    origin_key,
                );
                let idx = self.arena.push_get_idx(new_node);
                self.node_map.insert(key.to_string(), idx);
                (idx, true)
            }
        }
    }

    /// Create the arena and populate it.
    ///
    /// Creates the arena and populates it with all nodes from `all_nodes` that
    /// are part of `selection`. Nodes appearing in dependencies will eventually
    /// also become part of the graph.
    pub fn populate(
        all_nodes: &'a [S],
        selection: &'a [&str],
    ) -> (Self, Vec<usize>) {
        let mut arena = Vec::new();
        let mut node_map = HashMap::new();

        // Holds the index of each node in the original container
        let source_array_index: HashMap<&'a str, usize> = all_nodes
            .iter()
            .enumerate()
            .map(|(idx, t)| (t.key_ref(), idx))
            .collect();

        // Adds each node in the edge map with an empty Vec as the value.
        let edge_map =
            all_nodes.iter().map(|n| (n.key(), Vec::new())).collect();

        // Pushes all items selected in the arena and returns a Vec of their
        // indices.
        let selected_node_indices: Vec<usize> = selection
            .iter()
            .map(|s| {
                let idx = arena.push_get_idx(Self::new_node(
                    s.to_string(),
                    &all_nodes[source_array_index[s]],
                    false,
                    None,
                    s.to_string(),
                ));
                node_map.insert(s.to_string(), idx);
                idx
            })
            .collect();

        let node_list = selected_node_indices.iter().copied().collect();

        let graph_arena = Self {
            source_array_index,
            arena,
            node_map,
            node_list,
            edge_map,
        };

        (graph_arena, selected_node_indices)
    }

    /// Dispose the arena into an edge map and a list of nodes.
    #[allow(clippy::type_complexity)] // Inherent impls associated types?
    pub fn dispose(
        self,
    ) -> (
        HashMap<String, Vec<DependencyNode<&'a S, M>>>,
        Vec<DependencyNode<&'a S, M>>,
    ) {
        // Convert the index backed map and list to the actual dependency nodes.
        let edge_map = self.edge_map.from_index_backed(&self.arena);
        let node_list = self.node_list.from_index_backed(&self.arena);
        (edge_map, node_list)
    }

    fn new_node(
        key: String,
        value: &S,
        is_weak: bool,
        marker: Option<M>,
        origin: String,
    ) -> DependencyNode<&S, M> {
        let mut origin_nodes = HashSet::new();
        origin_nodes.insert(origin);

        DependencyNode {
            key,
            value,
            is_weak,
            marker,
            origin_nodes,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::client::module::*;
    use std::convert::TryInto;

    fn eq_lists<T>(a: &[T], b: &[T]) -> bool
    where
        T: PartialEq + Ord,
    {
        let mut a: Vec<_> = a.iter().collect();
        let mut b: Vec<_> = b.iter().collect();
        a.sort();
        b.sort();

        a == b
    }

    fn make_module(
        name: &str,
        dependencies: Vec<&str>,
        ordered_dependencies: Vec<&str>,
        inverse: Vec<&str>,
        after: Vec<&str>,
    ) -> ModuleDefinition {
        ModuleDefinition {
            name: name.to_string(),
            kind: ModuleKind::Service,
            inner: InnerDefinition::Service(ServiceOrTaskDefinition::new(
                name.to_string(),
                vec!["dummy".to_string()],
                None,
                HashMap::new(),
                HashMap::new(),
                None,
                dependencies.iter().map(|s| s.to_string()).collect(),
                ordered_dependencies.iter().map(|s| s.to_string()).collect(),
                after.iter().map(|s| s.to_string()).collect(),
                inverse.iter().map(|s| s.to_string()).collect(),
                vec![],
                None,
                vec![],
                TermSignal::KILL,
                false,
                None,
                None,
            )),
        }
    }

    fn is_before(m1: &str, m2: &str, elems: &Vec<&str>) -> bool {
        let mut index_a: i64 = -1;
        let mut index_b: i64 = -1;
        for (idx, el) in elems.into_iter().enumerate() {
            if *el == m1 {
                index_a = idx.try_into().unwrap();
            }
            if *el == m2 {
                index_b = idx.try_into().unwrap();
            }

            if index_a >= 0 && index_b >= 0 {
                return index_a < index_b;
            }
        }
        return false;
    }

    #[test]
    fn test_dependency_graph_sort() {
        let m1 = make_module("m1", vec!["m3", "m6"], vec![], vec![], vec![]);
        let m2 = make_module("m2", vec![], vec!["m4", "m5"], vec![], vec![]);
        let m3 = make_module("m3", vec!["m7"], vec![], vec!["m9"], vec![]);
        let m4 = make_module("m4", vec!["m7"], vec![], vec![], vec![]);
        let m5 = make_module("m5", vec![], vec![], vec![], vec![]);
        let m6 = make_module("m6", vec![], vec![], vec![], vec![]);
        let m7 = make_module("m7", vec!["m8"], vec![], vec![], vec![]);
        let m8 = make_module("m8", vec![], vec![], vec![], vec![]);
        let m9 = make_module("m9", vec!["m8"], vec![], vec![], vec![]);
        let modules = vec![m1, m2, m3, m4, m5, m6, m7, m8, m9];
        let selected =
            vec!["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9"];

        let graph = DependencyGraph::from(&modules, &selected);
        let result: Vec<Vec<&str>> = graph
            .group_sort()
            .unwrap()
            .groups
            .iter()
            .map(|g| g.iter().map(|v| &v.value.name[..]).collect::<Vec<_>>())
            .collect();

        assert!(
            result
                == vec![
                    vec!["m8"],
                    vec!["m7"],
                    vec!["m4"],
                    vec!["m5", "m6", "m3"],
                    vec!["m1", "m2", "m9"],
                ]
        );
    }

    #[test]
    fn test_dependency_graph_group_sort() {
        let m1 = make_module("m1", vec!["m3", "m6"], vec![], vec![], vec![]);
        let m2 = make_module("m2", vec![], vec!["m4", "m5"], vec![], vec![]);
        let m3 = make_module("m3", vec!["m7"], vec![], vec!["m9"], vec![]);
        let m4 = make_module("m4", vec!["m7"], vec![], vec![], vec![]);
        let m5 = make_module("m5", vec![], vec![], vec![], vec![]);
        let m6 = make_module("m6", vec![], vec![], vec![], vec![]);
        let m7 = make_module("m7", vec!["m8"], vec![], vec![], vec![]);
        let m8 = make_module("m8", vec![], vec![], vec![], vec![]);
        let m9 = make_module("m9", vec!["m8"], vec![], vec![], vec![]);
        let modules = vec![m1, m2, m3, m4, m5, m6, m7, m8, m9];
        let selected =
            vec!["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9"];

        let graph = DependencyGraph::from(&modules, &selected);
        let result: Vec<&str> = graph
            .dependency_sort()
            .unwrap()
            .iter()
            .map(|v| &v.value.name[..])
            .collect();

        assert!(is_before("m8", "m7", &result));
        assert!(is_before("m7", "m3", &result));
        assert!(is_before("m7", "m4", &result));
        assert!(is_before("m4", "m2", &result));
        assert!(is_before("m5", "m2", &result));
        assert!(is_before("m3", "m1", &result));
        assert!(is_before("m4", "m5", &result));
        assert!(is_before("m6", "m1", &result));
        assert!(is_before("m8", "m9", &result));
        assert!(is_before("m3", "m9", &result));
    }

    #[test]
    fn test_partial_dependency_graph_sort() {
        let m1 = make_module("m1", vec!["m3", "m6"], vec![], vec![], vec![]);
        let m2 = make_module("m2", vec!["m4", "m5"], vec![], vec![], vec![]);
        let m3 = make_module("m3", vec!["m7"], vec![], vec!["m9"], vec![]);
        let m4 = make_module("m4", vec!["m7"], vec![], vec![], vec![]);
        let m5 = make_module("m5", vec![], vec![], vec![], vec![]);
        let m6 = make_module("m6", vec![], vec![], vec![], vec![]);
        let m7 = make_module("m7", vec!["m8"], vec![], vec![], vec![]);
        let m8 = make_module("m8", vec![], vec![], vec![], vec!["m10"]);
        let m9 = make_module("m9", vec![], vec![], vec![], vec![]);
        let m10 = make_module("m10", vec![], vec![], vec![], vec!["m11"]);
        let m11 = make_module("m11", vec![], vec![], vec![], vec![]);
        let m12 = make_module("m12", vec![], vec![], vec![], vec!["m11"]);
        let modules = vec![m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12];
        let selected = vec!["m3", "m2", "m10", "m11"];

        let graph = DependencyGraph::from(&modules, &selected);
        let result: Vec<&str> = graph
            .dependency_sort()
            .unwrap()
            .iter()
            .map(|v| &v.value.name[..])
            .collect();

        let expected_items =
            vec!["m3", "m7", "m8", "m4", "m2", "m5", "m9", "m10", "m11"];
        assert!(eq_lists(&result, &expected_items));

        assert!(is_before("m8", "m7", &result));
        assert!(is_before("m7", "m3", &result));
        assert!(is_before("m7", "m4", &result));
        assert!(is_before("m4", "m2", &result));
        assert!(is_before("m5", "m2", &result));
        assert!(is_before("m3", "m9", &result));
        assert!(is_before("m10", "m8", &result));
        assert!(is_before("m11", "m10", &result));
    }
}
