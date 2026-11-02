//! Entry and byte bounded LRU; pinned renderer resources use separate budgets.
use std::{collections::HashMap, hash::Hash};
pub struct BudgetCache<K, V> {
    values: HashMap<K, (V, usize, u64)>,
    clock: u64,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
}
impl<K: Eq + Hash + Clone, V> BudgetCache<K, V> {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            values: HashMap::new(),
            clock: 0,
            bytes: 0,
            max_entries,
            max_bytes,
        }
    }
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock = self.clock.wrapping_add(1);
        self.values.get_mut(key).map(|v| {
            v.2 = self.clock;
            &v.0
        })
    }
    pub fn insert(&mut self, key: K, value: V, bytes: usize) {
        if let Some((_, size, _)) = self.values.remove(&key) {
            self.bytes -= size;
        }
        if bytes > self.max_bytes || self.max_entries == 0 {
            return;
        }
        while self.values.len() >= self.max_entries || self.bytes + bytes > self.max_bytes {
            let old = self
                .values
                .iter()
                .min_by_key(|(_, v)| v.2)
                .map(|(k, _)| k.clone());
            if let Some(k) = old {
                if let Some((_, n, _)) = self.values.remove(&k) {
                    self.bytes -= n;
                }
            } else {
                break;
            }
        }
        self.clock = self.clock.wrapping_add(1);
        self.bytes += bytes;
        self.values.insert(key, (value, bytes, self.clock));
    }
    pub fn clear(&mut self) {
        self.values.clear();
        self.bytes = 0;
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
