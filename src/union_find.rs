use std::iter::FromIterator;
use crate::serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnionFindGeneric<NodeType: UnionNodeTrait> {
    /// tree structure, each node has a parent
    pub link_parent: Vec<usize>,
    /// the node information, has the same length as `link_parent`
    pub payload: Vec<NodeType>,
    /// internal cache of parent list when calling `find`
    find_parent_list: Vec<usize>,
}

pub trait UnionNodeTrait {
    /// return `is_left`, `after_union`
    fn union(left: &Self, right: &Self) -> (bool, Self);
    /// clear the state, if [`UnionFindGeneric::clear`] is called then this must be provided
    fn clear(&mut self) { unimplemented!("[`UnionNodeTrait::clear`] must be implemented") }
    /// default structure
    fn default() -> Self where Self: std::marker::Sized { unimplemented!("[`UnionNodeTrait::default`] must be implemented") }
}

pub type ExampleUnionFind = UnionFindGeneric<ExampleUnionNode>;


/// define your own union-find node data structure like this
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExampleUnionNode {
    pub set_size: usize,
}

/// example trait implementation
impl UnionNodeTrait for ExampleUnionNode {
    #[inline]
    fn union(left: &Self, right: &Self) -> (bool, Self) {
        let result = Self {
            set_size: left.set_size + right.set_size,
        };
        // if left size is larger, choose left (weighted union)
        (left.set_size >= right.set_size, result)
    }
    #[inline]
    fn clear(&mut self) {
        self.set_size = 1;
    }
    #[inline]
    fn default() -> Self {
        Self {
            set_size: 1,
        }
    }
}

impl<NodeType: UnionNodeTrait> FromIterator<NodeType> for UnionFindGeneric<NodeType> {
    #[inline]
    fn from_iter<T: IntoIterator<Item = NodeType>>(iterator: T) -> Self {
        let mut uf = Self {
            link_parent: vec![],
            payload: vec![],
            find_parent_list: Vec::new(),
        };
        uf.extend(iterator);
        uf
    }
}

impl<NodeType: UnionNodeTrait> Extend<NodeType> for UnionFindGeneric<NodeType> {
    #[inline]
    fn extend<T: IntoIterator<Item = NodeType>>(&mut self, iterable: T) {
        let len = self.payload.len();
        let payload = iterable.into_iter();
        self.payload.extend(payload);

        let new_len = self.payload.len();
        self.link_parent.extend(len..new_len);

        self.find_parent_list.reserve(self.link_parent.len());
    }
}

impl<NodeType: UnionNodeTrait> UnionFindGeneric<NodeType> {
    #[inline]
    #[allow(dead_code)]
    pub fn new(len: usize) -> Self {
        Self::from_iter((0..len).map(|_| NodeType::default()))
    }

    #[inline]
    #[allow(dead_code)]
    pub fn size(&self) -> usize {
        self.payload.len()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn insert(&mut self, data: NodeType) -> usize {
        let key = self.payload.len();
        self.link_parent.push(key);
        self.payload.push(data);
        key
    }

    #[inline]
    pub fn union(&mut self, key0: usize, key1: usize) -> bool {
        let k0 = self.find(key0);
        let k1 = self.find(key1);
        if k0 == k1 {
            return false;
        }

        let (parent, child, val) = match NodeType::union(&self.payload[k0], &self.payload[k1]) {
            (true, val) => (k0, k1, val),  // left
            (false, val) => (k1, k0, val),  // right
        };
        self.payload[parent] = val;
        self.link_parent[child] = parent;

        true
    }

    #[inline]
    pub fn find(&mut self, key: usize) -> usize {
        let mut k = key;
        let mut p = self.link_parent[k];
        while p != k {
            self.find_parent_list.push(k);
            k = p;
            p = self.link_parent[p];
        }
        let root = k;
        for k in self.find_parent_list.iter() {
            self.link_parent[*k] = root;  // path compression
        }
        self.find_parent_list.clear();
        root
    }

    #[inline]
    pub fn immutable_find(&self, key: usize) -> usize {
        let mut k = key;
        let mut p = self.link_parent[k];
        while p != k {
            k = p;
            p = self.link_parent[p];
        }
        k
    }

    #[inline]
    pub fn get(&mut self, key: usize) -> &NodeType {
        let root_key = self.find(key);
        &self.payload[root_key]
    }

    #[inline]
    pub fn immutable_get(&self, key: usize) -> &NodeType {
        let root_key = self.immutable_find(key);
        &self.payload[root_key]
    }

    #[inline]
    pub fn get_mut(&mut self, key: usize) -> &mut NodeType {
        let root_key = self.find(key);
        &mut self.payload[root_key]
    }

    pub fn clear(&mut self) {
        debug_assert!(self.payload.len() == self.link_parent.len());
        for i in 0..self.link_parent.len() {
            self.link_parent[i] = i;
            let node = &mut self.payload[i];
            node.clear();
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_find_algorithm_test_1() {  // cargo test union_find_algorithm_test_1 -- --nocapture
        let mut uf = ExampleUnionFind::new(100);
        // test from https://github.com/gifnksm/union-find-rs/blob/master/src/tests.rs
        assert_eq!(1, uf.get(0).set_size);
        assert_eq!(1, uf.get(1).set_size);
        assert!(uf.find(0) != uf.find(1));
        assert!(uf.immutable_find(0) != uf.immutable_find(1));
        assert!(uf.find(1) != uf.find(2));
        assert!(uf.immutable_find(1) != uf.immutable_find(2));
        assert!(uf.union(0, 1));
        assert!(uf.find(0) == uf.find(1));
        assert!(uf.immutable_find(0) == uf.immutable_find(1));
        assert_eq!(2, uf.get(0).set_size);
        assert_eq!(2, uf.get(1).set_size);
        assert_eq!(1, uf.get(2).set_size);
        assert!(!uf.union(0, 1));
        assert_eq!(2, uf.get(0).set_size);
        assert_eq!(2, uf.get(1).set_size);
        assert_eq!(1, uf.get(2).set_size);
        assert!(uf.union(1, 2));
        assert_eq!(3, uf.get(0).set_size);
        assert_eq!(3, uf.get(1).set_size);
        assert_eq!(3, uf.get(2).set_size);
        assert!(uf.immutable_find(0) == uf.immutable_find(1));
        assert!(uf.find(0) == uf.find(1));
        assert!(uf.immutable_find(2) == uf.immutable_find(1));
        assert!(uf.find(2) == uf.find(1));
        let k100 = uf.insert(ExampleUnionNode::default());
        assert_eq!(k100, 100);
        let _ = uf.union(k100, 0);
        assert_eq!(4, uf.get(100).set_size);
        assert_eq!(101, uf.size());
        uf.clear();
    }
    
}
