use std::collections::HashMap;
use std::hash::Hash;

pub trait VecExt {
    type Item;

    /// Insert an entry into the Vec and return its index.
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
    /// Construct a new container by replacing elements pointed by the indices
    /// in `self` with the corresponding target elements in `container`.
    fn replace_indices_from(&self, container: &[V]) -> R;
}

pub trait FromOwnedIndexContainer<V, R> {
    /// Construct a new container by replacing elements pointed by the indices
    /// in `self` with the corresponding target elements in `container`.
    fn replace_indices_from(self, container: &[V]) -> R;
}

impl<V: Clone> FromIndexContainer<V, Vec<V>> for Vec<usize> {
    /// Contruct a new `Vec` with each index replaced by the item its pointing
    /// to in `container`.
    ///
    /// Rebuilds a `Vec<usize>` to `Vec<V>` where `V` is the type of the
    /// elements in `container`. For every index `i`, the new Vec will contain
    /// `container[i].clone()`.
    ///
    /// # Arguments:
    /// * `self` - A vector containing indices to the index backed container.
    /// * `container` - The container containing the elements pointed by the
    ///   indices in `vec`.
    fn replace_indices_from(&self, container: &[V]) -> Vec<V> {
        self.iter().map(|idx| container[*idx].clone()).collect()
    }
}

impl<K, V> FromOwnedIndexContainer<V, HashMap<K, Vec<V>>>
    for HashMap<K, Vec<usize>>
where
    K: Eq + Hash,
    V: Clone,
{
    /// Contruct a new `HashMap` with each index in the map values `Vec` replaced by
    /// the item its pointing to in `container`
    ///
    /// Rebuilds a `HashMap<K, Vec<usize>>` to a `HashMap<K, V>` where `V` is the type
    /// of elements in `container`. For every index `i`, the new Vec will contain
    /// `container[i].clone()`.
    ///
    /// The map's keys are consumed and moved as they are.
    ///
    /// # Arguments:
    /// * `self` - A map containing indices to the index backed container.
    /// * `container` - The container containing the elements pointed by the
    ///   indices in `vec`.
    fn replace_indices_from(self, container: &[V]) -> HashMap<K, Vec<V>> {
        self.into_iter()
            .map(|(k, v)| {
                (k, v.iter().map(|idx| container[*idx].clone()).collect())
            })
            .collect()
    }
}
