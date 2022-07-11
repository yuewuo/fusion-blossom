//! Dual Module
//! 
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

use super::util::*;
use std::sync::Arc;
use crate::derivative::Derivative;
use crate::parking_lot::RwLock;


/// A dual node is either a blossom or a vertex
#[derive(Derivative)]
#[derivative(Debug)]
pub enum DualNodeClass {
    Blossom {
        nodes_circle: Vec<DualNodePtr>,
    },
    SyndromeVertex {
        syndrome_index: VertexIndex,
    },
}

/// Three possible states: Grow (+1), Stay (+0), Shrink (-1)
#[derive(Derivative, PartialEq)]
#[derivative(Debug)]
pub enum DualNodeGrowState {
    Grow,
    Stay,
    Shrink,
}

/// gives the maximum absolute length to grow, if not possible, give the reason
#[derive(Derivative, PartialEq)]
#[derivative(Debug)]
pub enum MaxUpdateLength {
    /// non-zero maximum update length
    NonZeroGrow(Weight),
    /// conflicting growth
    Conflicting(DualNodePtr, DualNodePtr),
    /// conflicting growth because of touching virtual node
    TouchingVirtual(DualNodePtr, VertexIndex),
    /// blossom hitting 0 dual variable while shrinking
    BlossomNeedExpand(DualNodePtr),
    /// node hitting 0 dual variable while shrinking: note that this should have the lowest priority, normally it won't show up in a normal primal module
    VertexShrinkStop(DualNodePtr),
    /// no more nodes to constrain: no growing or shrinking
    NoMoreNodes,
}

/// A dual node corresponds to either a vertex or a blossom (on which the dual variables are defined)
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNode {
    /// the index of this dual node, helps to locate internal details of this dual node
    index: NodeIndex,
    /// the implementation internal node, providing the index of it
    pub internal: Option<usize>,
    /// the class of this dual node
    pub class: DualNodeClass,
    /// whether it grows, stays or shrinks
    pub grow_state: DualNodeGrowState,
    /// parent blossom: when parent exists, grow_state should be [`DualNodeGrowState::Stay`]
    pub parent_blossom: Option<DualNodePtr>,
}

/// the shared pointer of [`DualNode`]
pub struct DualNodePtr { ptr: Arc<RwLock<DualNode>>, }

impl RwLockPtr<DualNode> for DualNodePtr {
    fn new_ptr(ptr: Arc<RwLock<DualNode>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: DualNode) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<DualNode>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<DualNode>> { &mut self.ptr }
}

impl PartialEq for DualNodePtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl std::fmt::Debug for DualNodePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dual_node = self.read_recursive();
        write!(f, "{}", dual_node.index)
    }
}

impl DualNodePtr {
    /// helper function to set grow state with sanity check
    pub fn set_grow_state(&self, grow_state: DualNodeGrowState) {
        let mut dual_node = self.write();
        assert!(dual_node.parent_blossom.is_none(), "setting node grow state inside a blossom forbidden");
        dual_node.grow_state = grow_state;
    }
}

/// a sharable array of dual nodes, supporting dynamic partitioning;
/// note that a node can be destructed and we do not reuse its index, leaving a blank space
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualModuleInterface {
    /// all the dual node that can be used to control a concrete dual module implementation
    pub nodes: Vec<Option<DualNodePtr>>,
}

/// common trait that must be implemented for each implementation of dual module
pub trait DualModuleImpl {

    /// create a new dual module
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self;

    /// clear all growth and existing dual nodes, prepared for the next decoding
    fn clear(&mut self);

    /// add corresponding dual node, note that [`DualNode.internal`] must be None, i.e. each dual node must be created exactly once
    fn add_dual_node(&mut self, node: DualNodePtr);

    #[inline(always)]
    /// helper function to specifically add a syndrome node
    fn add_syndrome_node(&mut self, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::SyndromeVertex{ .. })
        }, "node class mismatch");
        self.add_dual_node(node)
    }

    #[inline(always)]
    /// helper function to specifically add a blossom node
    fn add_blossom(&mut self, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::Blossom{ .. })
        }, "node class mismatch");
        self.add_dual_node(node)
    }

    /// remove a blossom, note that this dual node ptr is already expanded from the root: normally you only need to remove this blossom
    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr);

    /// An optional function that helps to break down the implementation of [`DualModuleImpl::compute_maximum_update_length`]
    /// check the maximum length to grow (shrink) specific dual node, if length is 0, give the reason of why it cannot further grow (shrink).
    /// if `is_grow` is false, return `length` <= 0, in any case |`length`| is maximized so that at least one edge becomes fully grown or fully not-grown.
    /// if `simultaneous_update` is true, also check for the peer node according to [`DualNode::grow_state`].
    fn compute_maximum_update_length_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _is_grow: bool, _simultaneous_update: bool) -> MaxUpdateLength {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// check the maximum length to grow (shrink) for all nodes
    fn compute_maximum_update_length(&mut self) -> MaxUpdateLength;

    /// An optional function that can manipulate individual dual node, not necessarily supported by all implementations
    fn grow_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _length: Weight) {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// grow a specific length globally, length must be positive.
    /// note that reversing the process is possible, but not recommended: to do that, reverse the state of each dual node, Grow->Shrink, Shrink->Grow
    fn grow(&mut self, length: Weight);

}

impl DualModuleInterface {

    pub fn new(syndrome: &Vec<VertexIndex>, dual_module_impl: &mut impl DualModuleImpl) -> Self {
        let mut array = Self {
            nodes: Vec::new(),
        };
        for vertex_idx in syndrome.iter() {
            array.create_syndrome_node(*vertex_idx, dual_module_impl);
        }
        array
    }

    /// create a dual node corresponding to a syndrome vertex
    pub fn create_syndrome_node(&mut self, vertex_idx: VertexIndex, dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        let node_idx = self.nodes.len();
        let node_ptr = DualNodePtr::new(DualNode {
            index: node_idx,
            internal: None,
            class: DualNodeClass::SyndromeVertex {
                syndrome_index: vertex_idx,
            },
            grow_state: DualNodeGrowState::Grow,
            parent_blossom: None,
        });
        self.nodes.push(Some(node_ptr.clone()));
        dual_module_impl.add_syndrome_node(node_ptr.clone());
        node_ptr
    }

    /// create a dual node corresponding to a blossom, automatically set the grow state of internal nodes;
    /// the nodes circle MUST starts with a growing node and ends with a shrinking node
    pub fn create_blossom(&mut self, nodes_circle: Vec<DualNodePtr>, dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        let node_idx = self.nodes.len();
        let node_ptr = DualNodePtr::new(DualNode {
            index: node_idx,
            internal: None,
            class: DualNodeClass::Blossom {
                nodes_circle: Vec::new(),  // will fill in it later, after all nodes have been checked
            },
            grow_state: DualNodeGrowState::Grow,
            parent_blossom: None,
        });
        for (i, node) in nodes_circle.iter().enumerate() {
            let mut node = node.write();
            assert!(node.parent_blossom.is_none(), "cannot create blossom on a node that already belongs to a blossom");
            assert!(&node.grow_state == (if i % 2 == 0 { &DualNodeGrowState::Grow } else { &DualNodeGrowState::Shrink })
                , "the nodes circle MUST starts with a growing node and ends with a shrinking node");
            node.grow_state = DualNodeGrowState::Stay;
            node.parent_blossom = Some(node_ptr.clone());
        }
        {  // fill in the nodes because they're in a valid state (all linked to this blossom)
            let mut node = node_ptr.write();
            node.class = DualNodeClass::Blossom {
                nodes_circle: nodes_circle,
            };
            self.nodes.push(Some(node_ptr.clone()));
        }
        dual_module_impl.add_blossom(node_ptr.clone());
        node_ptr
    }

    /// expand a blossom: note that different from Blossom V library, we do not maintain tree structure after a blossom is expanded;
    /// this is because we're growing all trees together, and due to the natural of quantum codes, this operation is not likely to cause
    /// bottleneck as long as physical error rate is well below the threshold. All internal nodes will have a [`DualNodeGrowState::Stay`] state afterwards.
    pub fn expand_blossom(&mut self, dual_node_ptr: DualNodePtr, dual_module_impl: &mut impl DualModuleImpl) {
        let node = dual_node_ptr.read_recursive();
        let node_idx = node.index;
        assert!(self.nodes[node_idx].is_some(), "the blossom should not be expanded before");
        assert!(self.nodes[node_idx].as_ref().unwrap() == &dual_node_ptr, "the blossom doesn't belong to this DualModuleInterface");
        self.nodes[node_idx] = None;  // remove this blossom from root
        match &node.class {
            DualNodeClass::Blossom { nodes_circle } => {
                for node in nodes_circle.iter() {
                    let mut node = node.write();
                    assert!(node.parent_blossom.is_some() && node.parent_blossom.as_ref().unwrap() == &dual_node_ptr, "internal error: parent blossom must be this blossom");
                    assert!(&node.grow_state == &DualNodeGrowState::Stay, "internal error: children node must be DualNodeGrowState::Stay");
                    node.parent_blossom = None;
                }
            },
            _ => { unreachable!() }
        }
        dual_module_impl.remove_blossom(dual_node_ptr.clone());
    }

}

impl MaxUpdateLength {

    /// get the minimum update length of all individual maximum update length;
    /// if any length is zero, then also choose one reason with highest priority
    pub fn min(a: Self, b: Self) -> Self {
        match (&a, &b) {
            // if any of them is default, then take the other
            (_, MaxUpdateLength::NoMoreNodes) => { a },
            (MaxUpdateLength::NoMoreNodes, _) => { b },
            // if both of them is non-zero, then take the smaller one
            (MaxUpdateLength::NonZeroGrow(length_1), MaxUpdateLength::NonZeroGrow(length_2)) => {
                if length_1 < length_2 { a } else { b }
            },
            // TODO: complex priority
            (MaxUpdateLength::Conflicting( .. ), _) => { a },
            (_, MaxUpdateLength::Conflicting( .. )) => { b },
            // VertexShrinkStop has the lowest priority
            (MaxUpdateLength::NonZeroGrow(_), MaxUpdateLength::VertexShrinkStop(_)) => { a },
            (MaxUpdateLength::VertexShrinkStop(_), MaxUpdateLength::NonZeroGrow(_)) => { b },
            _ => {
                unimplemented!("min of {:?} and {:?}", a, b)
            }
        }
    }

    /// useful function to assert expected case
    #[allow(dead_code)]
    pub fn is_conflicting(&self, a: &DualNodePtr, b: &DualNodePtr) -> bool {
        if let MaxUpdateLength::Conflicting(n1, n2) = self {
            if n1 == a && n2 == b {
                return true
            }
            if n1 == b && n2 == a {
                return true
            }
        }
        false
    }

}
