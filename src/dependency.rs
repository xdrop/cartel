use crate::collections::{FromIndexContainer, FromOwnedIndexContainer, VecExt};
use anyhow::{bail, Result};
use std::collections::HashMap;
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
    pub key: String,
    pub value: T,
    pub marker: M,
}

pub struct DependencyEdge<M: PartialOrd> {
    /// The key of the node this edge points to.
    pub edge_ptr: String,
    /// A marker is used to mark the node that this edge points to. Since
    /// multiple edges may be pointing to the same node with potentially
    /// different markers, the marker type must implement [PartialOrd].
    pub edge_marker: M,
}

pub trait WithDependencies<M: PartialOrd> {
    fn key(&self) -> String;
    fn key_ref(&self) -> &str;
    fn dependencies(&self) -> Vec<DependencyEdge<M>>;
}

impl<T: Copy, M: Copy> Clone for DependencyNode<T, M> {
    fn clone(&self) -> Self {
        DependencyNode {
            key: self.key.clone(),
            value: self.value,
            marker: self.marker,
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
    pub fn from<'b>(
        src: &'a [T],
        selected: &'b [&str],
    ) -> DependencyGraph<'a, T, M> {
        // Holds the index of each key in Vec<T>
        let pos_index: HashMap<&str, usize> = src
            .iter()
            .enumerate()
            .map(|(idx, t)| (t.key_ref(), idx))
            .collect();

        let mut raw_entries: Vec<DependencyNode<&T, M>> = Vec::new();
        // Contains the edges of each node as indices in raw_entries
        let mut edge_map: HashMap<String, Vec<usize>> = HashMap::new();
        // Map each node to its index in raw_entries
        let mut node_map: HashMap<String, usize> = HashMap::new();

        let node_list: Vec<usize> = selected
            .iter()
            .map(|s| {
                raw_entries.push_get_idx(Self::new_node(
                    s.to_string(),
                    &src[pos_index[s]],
                    M::default(),
                ))
            })
            .collect();

        let mut node_stack: Vec<usize> = node_list.iter().copied().collect();

        while !node_stack.is_empty() {
            let node_idx = node_stack.pop().unwrap();

            let (node_key, dependencies) = {
                let node = &raw_entries[node_idx];
                (node.key.clone(), node.value.dependencies())
            };

            if node_map.get(&node_key).is_none() {
                node_map.insert(node_key.clone(), node_idx);
            }

            if edge_map.get(&node_key).is_none() {
                edge_map.insert(node_key.clone(), Vec::new());
            }

            dependencies.iter().for_each(|edge| {
                let ref edge_ptr = edge.edge_ptr;
                let marker = edge.edge_marker;
                let dep_item = &src[pos_index[edge_ptr.as_str()]];

                let pointed_to = match node_map.get(edge_ptr.as_str()) {
                    Some(idx) => {
                        Self::maybe_upgrade_marker(
                            marker,
                            &mut raw_entries[*idx],
                        );
                        *idx
                    }
                    None => {
                        let new_node =
                            Self::new_node(edge_ptr.clone(), dep_item, marker);
                        let idx = raw_entries.push_get_idx(new_node);
                        node_map.insert(edge_ptr.clone(), idx);
                        node_stack.push(idx);
                        idx
                    }
                };

                #[rustfmt::skip]
                edge_map.get_mut(node_key.as_str()).unwrap().push(pointed_to);
            })
        }

        // Convert the index backed map and list to the actual dependency nodes.
        DependencyGraph {
            edge_map: edge_map.from_index_backed(&raw_entries),
            node_list: node_list.from_index_backed(&raw_entries),
        }
    }

    /// Upgrade the current marker on the dependency node.
    ///
    /// Checks the existing marker on the node, if the new marker is higher
    /// ranked than the current one, then the node is marked with the new marker
    /// instead.
    fn maybe_upgrade_marker(new_marker: M, node: &mut DependencyNode<&T, M>) {
        if new_marker > node.marker {
            node.marker = new_marker;
        }
    }

    fn new_node(key: String, value: &T, marker: M) -> DependencyNode<&T, M> {
        DependencyNode { key, value, marker }
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

    fn make_module(name: &str, dependencies: Vec<&str>) -> ModuleDefinition {
        ModuleDefinition {
            name: name.to_string(),
            kind: ModuleKind::Service,
            inner: InnerDefinition::Service(ServiceOrTaskDefinition::new(
                name.to_string(),
                vec!["dummy".to_string()],
                HashMap::new(),
                None,
                dependencies.iter().map(|s| s.to_string()).collect(),
                None,
                vec![],
                TermSignal::KILL,
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
    fn test_dependency_graph() {
        let m1 = make_module("m1", vec!["m3", "m6"]);
        let m2 = make_module("m2", vec!["m4", "m5"]);
        let m3 = make_module("m3", vec!["m7"]);
        let m4 = make_module("m4", vec!["m7"]);
        let m5 = make_module("m5", vec![]);
        let m6 = make_module("m6", vec![]);
        let m7 = make_module("m7", vec!["m8"]);
        let m8 = make_module("m8", vec![]);
        let modules = vec![m1, m2, m3, m4, m5, m6, m7, m8];
        let selected = vec!["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8"];

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
        assert!(is_before("m6", "m1", &result));
    }

    #[test]
    fn test_dependency_graph_partial() {
        let m1 = make_module("m1", vec!["m3", "m6"]);
        let m2 = make_module("m2", vec!["m4", "m5"]);
        let m3 = make_module("m3", vec!["m7"]);
        let m4 = make_module("m4", vec!["m7"]);
        let m5 = make_module("m5", vec![]);
        let m6 = make_module("m6", vec![]);
        let m7 = make_module("m7", vec!["m8"]);
        let m8 = make_module("m8", vec![]);
        let modules = vec![m1, m2, m3, m4, m5, m6, m7, m8];
        let selected = vec!["m3", "m2"];

        let graph = DependencyGraph::from(&modules, &selected);
        let result: Vec<&str> = graph
            .dependency_sort()
            .unwrap()
            .iter()
            .map(|v| &v.value.name[..])
            .collect();

        let expected_items = vec!["m3", "m7", "m8", "m4", "m2", "m5"];
        assert!(eq_lists(&result, &expected_items));

        assert!(is_before("m8", "m7", &result));
        assert!(is_before("m7", "m3", &result));
        assert!(is_before("m7", "m4", &result));
        assert!(is_before("m4", "m2", &result));
        assert!(is_before("m5", "m2", &result));
    }
}
