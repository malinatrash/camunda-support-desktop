use std::collections::HashMap;
use std::hash::Hash;

pub(super) fn count_in_order<K: Eq + Hash + Clone>(keys: impl IntoIterator<Item = K>) -> Vec<(K, i32)> {
    let mut counts: Vec<(K, i32)> = Vec::new();
    let mut index: HashMap<K, usize> = HashMap::new();
    for key in keys {
        match index.get(&key) {
            Some(&position) => counts[position].1 += 1,
            None => {
                index.insert(key.clone(), counts.len());
                counts.push((key, 1));
            }
        }
    }
    counts
}

pub(super) fn group_in_order<T, K: Eq + Hash + Clone>(items: impl IntoIterator<Item = T>, key: impl Fn(&T) -> K) -> Vec<(K, Vec<T>)> {
    let mut groups: Vec<(K, Vec<T>)> = Vec::new();
    let mut index: HashMap<K, usize> = HashMap::new();
    for item in items {
        let item_key = key(&item);
        match index.get(&item_key) {
            Some(&position) => groups[position].1.push(item),
            None => {
                index.insert(item_key.clone(), groups.len());
                groups.push((item_key, vec![item]));
            }
        }
    }
    groups
}
