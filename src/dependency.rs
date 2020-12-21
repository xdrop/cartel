use simple_error::{bail, SimpleError};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

pub struct DependencyGraph<'a, T: WithDependencies + Eq + Hash> {
    edge_map: HashMap<String, Vec<DependencyNode<&'a T>>>,
    node_list: Vec<DependencyNode<&'a T>>,
}

#[derive(Debug)]
struct DependencyNode<T> {
    key: String,
    value: T,
}

pub trait WithDependencies {
    fn key(&self) -> String;
    fn key_ref(&self) -> &str;
    fn dependencies(&self) -> &Vec<String>;
}

impl<T: Copy> Clone for DependencyNode<T> {
    fn clone(&self) -> Self {
        DependencyNode {
            key: self.key.clone(),
            value: self.value,
        }
    }
}

impl<T> Hash for DependencyNode<T> {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.key.hash(state);
    }
}

impl<T> PartialEq for DependencyNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for DependencyNode<T> {}

enum MarkType {
    PERMANENT,
    TEMPORARY,
}

/// A dependency graph over type T.
impl<'a, T> DependencyGraph<'a, T>
where
    T: WithDependencies + Eq + Hash,
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
        src: &'a Vec<T>,
        selected: &'b Vec<&str>,
    ) -> DependencyGraph<'a, T> {
        // Holds the index of each key in Vec<T>
        let pos_index: HashMap<&str, usize> = src
            .iter()
            .enumerate()
            .map(|(idx, t)| (t.key_ref(), idx))
            .collect();

        let mut node_list: Vec<DependencyNode<&'a T>> = selected
            .iter()
            .map(|s| DependencyNode {
                key: s.to_owned().to_string(),
                value: &src[pos_index[s]],
            })
            .collect();

        let mut edge_map = HashMap::new();

        let mut stack: Vec<usize> =
            selected.iter().map(|k| pos_index[k]).collect();

        while stack.len() > 0 {
            let idx = stack.pop().unwrap();
            let item = &src[idx];

            if let None = edge_map.get(item.key_ref()) {
                edge_map.insert(item.key(), Vec::new());
            } else {
                continue;
            }

            item.dependencies().iter().for_each(|dep_key| {
                let dep_item = &src[pos_index[dep_key.as_str()]];
                let dependency_node = DependencyNode {
                    key: dep_key.clone(),
                    value: dep_item,
                };

                if let None = edge_map.get(dep_key) {
                    node_list.push(dependency_node.clone());
                    stack.push(pos_index[dep_key.as_str()]);
                }

                edge_map
                    .get_mut(item.key_ref())
                    .unwrap()
                    .push(dependency_node);
            })
        }

        DependencyGraph {
            edge_map,
            node_list,
        }
    }

    /// Return a sorted list of dependencies.
    ///
    /// Sorts dependencies so that dependent modules are deployed before the
    /// modules that depend on them. The topological sort is performed using
    /// modified DFS.
    pub fn dependency_sort(&self) -> Result<Vec<&T>, SimpleError> {
        let mut sorted = Vec::new();
        let mut stack: Vec<(bool, &DependencyNode<&T>)> = Vec::new();
        let mut marked: HashMap<&DependencyNode<&T>, MarkType> = HashMap::new();
        let mut unmarked: Vec<&DependencyNode<&T>> =
            Vec::from_iter(self.node_list.iter());

        // While we have still nodes unmarked
        while sorted.len() < self.node_list.len() {
            let to_mark = unmarked.pop().unwrap();
            stack.push((false, to_mark));

            while stack.len() > 0 {
                let (is_parent, node) = stack.pop().unwrap();

                if is_parent {
                    sorted.push(node.value);
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
    use crate::client::module::{ModuleDefinitionV1, ModuleKindV1};
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

    fn make_module(name: &str, dependencies: Vec<&str>) -> ModuleDefinitionV1 {
        ModuleDefinitionV1::new(
            ModuleKindV1::Service,
            name.to_string(),
            vec!["dummy".to_string()],
            HashMap::new(),
            None,
            dependencies.iter().map(|s| s.to_string()).collect(),
        )
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

        let graph =
            DependencyGraph::<ModuleDefinitionV1>::from(&modules, &selected);
        let result: Vec<&str> = graph
            .dependency_sort()
            .unwrap()
            .iter()
            .map(|v| &v.name[..])
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

        let graph =
            DependencyGraph::<ModuleDefinitionV1>::from(&modules, &selected);
        let result: Vec<&str> = graph
            .dependency_sort()
            .unwrap()
            .iter()
            .map(|v| &v.name[..])
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
