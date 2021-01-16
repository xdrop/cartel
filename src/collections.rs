use std::collections::HashMap;
use std::hash::Hash;

pub trait VecExt {
    type Item;

    fn push_get_idx(&mut self, item: Self::Item) -> usize;
}

impl<T> VecExt for Vec<T> {
    type Item = T;

    /// Insert an entry into the Vec and return its index.
    fn push_get_idx(&mut self, item: T) -> usize {
        let idx = self.len();
        self.push(item);
        idx
    }
}

pub trait FromIndexContainer<V, R> {
    fn from_index_backed(&self, container: &Vec<V>) -> R;
}

pub trait FromOwnedIndexContainer<V, R> {
    fn from_index_backed(self, container: &Vec<V>) -> R;
}

impl<V: Clone> FromIndexContainer<V, Vec<V>> for Vec<usize> {
    /// Rebuild a Vec of indices from its index backed container.
    ///
    /// Rebuilds a `Vec<usize>` to a `Vec<V>` where `V` is the type of element in
    /// the index backed container.
    ///
    /// # Arguments:
    /// * `vec` - A vector containing indices to the index backed container.
    /// * `container` - The container containing the elements pointed by the
    ///   indices in `vec`.
    fn from_index_backed(&self, container: &Vec<V>) -> Vec<V> {
        self.iter().map(|idx| container[*idx].clone()).collect()
    }
}

impl<K, V> FromOwnedIndexContainer<V, HashMap<K, Vec<V>>>
    for HashMap<K, Vec<usize>>
where
    K: Eq + Hash,
    V: Clone,
{
    /// Rebuild a HashMap of indices from its index backed container.
    ///
    /// Rebuilds a `HashMap<K, usize>` to a `HashMap<K, V>` where `V` is the type
    /// of element in the index backed container.
    ///
    /// The map's keys are passed through as is.
    ///
    /// # Arguments:
    /// * `map` - A map containing indices to the index backed container.
    /// * `container` - The container containing the elements pointed by the
    ///   indices in `vec`.
    fn from_index_backed(self, container: &Vec<V>) -> HashMap<K, Vec<V>> {
        self.into_iter()
            .map(|(k, v)| {
                (k, v.iter().map(|idx| container[*idx].clone()).collect())
            })
            .collect()
    }
}
