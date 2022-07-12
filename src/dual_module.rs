//! Dual Module
//! 
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

use super::util::*;
use std::sync::Arc;
use crate::derivative::Derivative;
use crate::parking_lot::RwLock;
use core::cmp::Ordering;
use std::collections::BinaryHeap;


/// A dual node is either a blossom or a vertex
#[derive(Derivative, Clone)]
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
#[derive(Derivative, PartialEq, Clone, Copy)]
#[derivative(Debug)]
pub enum DualNodeGrowState {
    Grow,
    Stay,
    Shrink,
}

/// gives the maximum absolute length to grow, if not possible, give the reason
#[derive(Derivative, PartialEq, Eq, Clone)]
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
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum GroupMaxUpdateLength {
    /// non-zero maximum update length
    NonZeroGrow(Weight),
    /// conflicting reasons
    Conflicts(BinaryHeap<MaxUpdateLength>),
}

impl GroupMaxUpdateLength {

    pub fn new() -> Self {
        Self::NonZeroGrow(Weight::MAX)
    }

    pub fn add(&mut self, max_update_length: MaxUpdateLength) {
        match self {
            Self::NonZeroGrow(current_length) => {
                if let MaxUpdateLength::NonZeroGrow(length) = max_update_length {
                    *current_length = std::cmp::min(*current_length, length);
                } else {
                    let mut heap = BinaryHeap::new();
                    heap.push(max_update_length);
                    *self = Self::Conflicts(heap);
                }
            },
            Self::Conflicts(conflicts) => {
                // only add conflicts, not NonZeroGrow
                if !matches!(max_update_length, MaxUpdateLength::NonZeroGrow(_)) {
                    conflicts.push(max_update_length)
                }
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::NonZeroGrow(Weight::MAX))
    }

    pub fn get_none_zero_growth(&self) -> Option<Weight> {
        match self {
            Self::NonZeroGrow(length) => {
                debug_assert!(*length != Weight::MAX, "please call GroupMaxUpdateLength::is_empty to check if this group is empty");
                Some(*length)
            },
            _ => { None }
        }
    }

    pub fn get_conflicts(&mut self) -> &mut BinaryHeap<MaxUpdateLength> {
        match self {
            Self::NonZeroGrow(_) => {
                panic!("please call GroupMaxUpdateLength::get_none_zero_growth to check if this group is none_zero_growth");
            },
            Self::Conflicts(conflicts) => { conflicts }
        }
    }

    pub fn get_conflicts_immutable(&self) -> &BinaryHeap<MaxUpdateLength> {
        match self {
            Self::NonZeroGrow(_) => {
                panic!("please call GroupMaxUpdateLength::get_none_zero_growth to check if this group is none_zero_growth");
            },
            Self::Conflicts(conflicts) => { conflicts }
        }
    }

}

/// A dual node corresponds to either a vertex or a blossom (on which the dual variables are defined)
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct DualNode {
    /// the index of this dual node, helps to locate internal details of this dual node
    pub index: NodeIndex,
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

impl Clone for DualNodePtr {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl RwLockPtr<DualNode> for DualNodePtr {
    fn new_ptr(ptr: Arc<RwLock<DualNode>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: DualNode) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<DualNode>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<DualNode>> { &mut self.ptr }
}

impl PartialEq for DualNodePtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl Eq for DualNodePtr { }

impl Ord for DualNodePtr {
    /// compare pointer address, just to have a consistent order between pointers
    fn cmp(&self, other: &Self) -> Ordering {
        let ptr1 = Arc::as_ptr(self.ptr());
        let ptr2 = Arc::as_ptr(other.ptr());
        // https://doc.rust-lang.org/reference/types/pointer.html
        // "When comparing raw pointers they are compared by their address, rather than by what they point to."
        ptr1.cmp(&ptr2)
    }
}

impl PartialOrd for DualNodePtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Debug for DualNodePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dual_node = self.read_recursive();
        write!(f, "{}", dual_node.index)
    }
}

impl DualNodePtr {

    /// helper function to set grow state with sanity check
    fn set_grow_state(&self, grow_state: DualNodeGrowState) {
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
    /// record the total growing nodes, should be non-negative in a normal running algorithm
    pub sum_grow_speed: Weight,
    /// record the total sum of dual variables
    pub sum_dual_variables: Weight,
    /// debug mode: only resolve one conflict each time
    pub debug_print_actions: bool,
}

/// common trait that must be implemented for each implementation of dual module
pub trait DualModuleImpl {

    /// create a new dual module
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self;

    /// clear all growth and existing dual nodes, prepared for the next decoding
    fn clear(&mut self);

    /// add corresponding dual node, note that [`DualNode.internal`] must be None, i.e. each dual node must be created exactly once
    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr);

    #[inline(always)]
    /// helper function to specifically add a syndrome node
    fn add_syndrome_node(&mut self, dual_node_ptr: &DualNodePtr) {
        debug_assert!({
            let node = dual_node_ptr.read_recursive();
            matches!(node.class, DualNodeClass::SyndromeVertex{ .. })
        }, "node class mismatch");
        self.add_dual_node(dual_node_ptr)
    }

    #[inline(always)]
    /// helper function to specifically add a blossom node
    fn add_blossom(&mut self, dual_node_ptr: &DualNodePtr) {
        debug_assert!({
            let node = dual_node_ptr.read_recursive();
            matches!(node.class, DualNodeClass::Blossom{ .. })
        }, "node class mismatch");
        self.add_dual_node(dual_node_ptr)
    }

    /// remove a blossom, note that this dual node ptr is already expanded from the root: normally you only need to remove this blossom
    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr);

    /// update grow state
    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState);

    /// An optional function that helps to break down the implementation of [`DualModuleImpl::compute_maximum_update_length`]
    /// check the maximum length to grow (shrink) specific dual node, if length is 0, give the reason of why it cannot further grow (shrink).
    /// if `is_grow` is false, return `length` <= 0, in any case |`length`| is maximized so that at least one edge becomes fully grown or fully not-grown.
    /// if `simultaneous_update` is true, also check for the peer node according to [`DualNode::grow_state`].
    fn compute_maximum_update_length_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _is_grow: bool, _simultaneous_update: bool) -> MaxUpdateLength {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// check the maximum length to grow (shrink) for all nodes, return a list of conflicting reason and a single number indicating the maximum length to grow:
    /// this number will be 0 if any conflicting reason presents
    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength;

    /// An optional function that can manipulate individual dual node, not necessarily supported by all implementations
    fn grow_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _length: Weight) {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// grow a specific length globally, length must be positive.
    /// note that reversing the process is possible, but not recommended: to do that, reverse the state of each dual node, Grow->Shrink, Shrink->Grow
    fn grow(&mut self, length: Weight);

}

impl DualModuleInterface {

    /// a dual module interface MUST be created given a concrete implementation of the dual module
    pub fn new(syndrome: &Vec<VertexIndex>, dual_module_impl: &mut impl DualModuleImpl) -> Self {
        let mut array = Self {
            nodes: Vec::new(),
            sum_grow_speed: 0,
            sum_dual_variables: 0,
            debug_print_actions: false,
        };
        dual_module_impl.clear();
        for vertex_idx in syndrome.iter() {
            array.create_syndrome_node(*vertex_idx, dual_module_impl);
        }
        array
    }

    pub fn create_syndrome_node(&mut self, vertex_idx: VertexIndex, dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        self.sum_grow_speed += 1;
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
        dual_module_impl.add_syndrome_node(&node_ptr);
        node_ptr
    }

    /// check whether a pointer belongs to this node, it will acquire a reader lock on `dual_node_ptr`
    pub fn check_ptr_belonging(&self, dual_node_ptr: &DualNodePtr) -> bool {
        let dual_node = dual_node_ptr.read_recursive();
        if dual_node.index >= self.nodes.len() { return false }
        if let Some(ptr) = self.nodes[dual_node.index].as_ref() {
            return ptr == dual_node_ptr
        } else {
            return false
        }
    }

    /// create a dual node corresponding to a blossom, automatically set the grow state of internal nodes;
    /// the nodes circle MUST starts with a growing node and ends with a shrinking node
    pub fn create_blossom(&mut self, nodes_circle: Vec<DualNodePtr>, dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        if self.debug_print_actions {
            eprintln!("[DualModuleInterface::create_blossom] {:?} -> {}", nodes_circle, self.nodes.len());
        }
        let blossom_node_ptr = DualNodePtr::new(DualNode {
            index: self.nodes.len(),
            internal: None,
            class: DualNodeClass::Blossom {
                nodes_circle: Vec::new(),
            },
            grow_state: DualNodeGrowState::Grow,
            parent_blossom: None,
        });
        for (i, node_ptr) in nodes_circle.iter().enumerate() {
            debug_assert!(self.check_ptr_belonging(node_ptr), "this ptr doesn't belong to this interface");
            let node = node_ptr.read_recursive();
            assert!(node.parent_blossom.is_none(), "cannot create blossom on a node that already belongs to a blossom");
            assert!(&node.grow_state == (if i % 2 == 0 { &DualNodeGrowState::Grow } else { &DualNodeGrowState::Shrink })
                , "the nodes circle MUST starts with a growing node and ends with a shrinking node");
            drop(node);
            // set state must happen before setting parent
            self.set_grow_state(node_ptr, DualNodeGrowState::Stay, dual_module_impl);
            // then update parent
            let mut node = node_ptr.write();
            node.parent_blossom = Some(blossom_node_ptr.clone());
        }
        {  // fill in the nodes because they're in a valid state (all linked to this blossom)
            let mut node = blossom_node_ptr.write();
            node.index = self.nodes.len();
            node.class = DualNodeClass::Blossom {
                nodes_circle: nodes_circle,
            };
            self.nodes.push(Some(blossom_node_ptr.clone()));
        }
        self.sum_grow_speed += 1;
        dual_module_impl.add_blossom(&blossom_node_ptr);
        blossom_node_ptr
    }

    /// expand a blossom: note that different from Blossom V library, we do not maintain tree structure after a blossom is expanded;
    /// this is because we're growing all trees together, and due to the natural of quantum codes, this operation is not likely to cause
    /// bottleneck as long as physical error rate is well below the threshold. All internal nodes will have a [`DualNodeGrowState::Grow`] state afterwards.
    pub fn expand_blossom(&mut self, blossom_node_ptr: DualNodePtr, dual_module_impl: &mut impl DualModuleImpl) {
        if self.debug_print_actions {
            eprintln!("[DualModuleInterface::expand_blossom] {:?}", blossom_node_ptr);
        }
        dual_module_impl.remove_blossom(blossom_node_ptr.clone());
        let node = blossom_node_ptr.read_recursive();
        match &node.grow_state {
            DualNodeGrowState::Grow => { self.sum_grow_speed += -1; },
            DualNodeGrowState::Shrink => { self.sum_grow_speed += 1; },
            DualNodeGrowState::Stay => { },
        }
        let node_idx = node.index;
        assert!(self.nodes[node_idx].is_some(), "the blossom should not be expanded before");
        assert!(self.nodes[node_idx].as_ref().unwrap() == &blossom_node_ptr, "the blossom doesn't belong to this DualModuleInterface");
        self.nodes[node_idx] = None;  // remove this blossom from root
        match &node.class {
            DualNodeClass::Blossom { nodes_circle } => {
                for node_ptr in nodes_circle.iter() {
                    let mut node = node_ptr.write();
                    assert!(node.parent_blossom.is_some() && node.parent_blossom.as_ref().unwrap() == &blossom_node_ptr, "internal error: parent blossom must be this blossom");
                    assert!(&node.grow_state == &DualNodeGrowState::Stay, "internal error: children node must be DualNodeGrowState::Stay");
                    node.parent_blossom = None;
                    drop(node);
                    self.set_grow_state(node_ptr, DualNodeGrowState::Grow, dual_module_impl);
                }
            },
            _ => { unreachable!() }
        }
    }

    /// a helper function to update grow state
    pub fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState, dual_module_impl: &mut impl DualModuleImpl) {
        if self.debug_print_actions {
            eprintln!("[DualModuleInterface::set_grow_state] {:?} {:?}", dual_node_ptr, grow_state);
        }
        {  // update sum_grow_speed
            let node = dual_node_ptr.read_recursive();
            match &node.grow_state {
                DualNodeGrowState::Grow => { self.sum_grow_speed -= 1; },
                DualNodeGrowState::Shrink => { self.sum_grow_speed += 1; },
                DualNodeGrowState::Stay => { },
            }
            match grow_state {
                DualNodeGrowState::Grow => { self.sum_grow_speed += 1; },
                DualNodeGrowState::Shrink => { self.sum_grow_speed -= 1; },
                DualNodeGrowState::Stay => { },
            }
        }
        dual_node_ptr.set_grow_state(grow_state);
        dual_module_impl.set_grow_state(&dual_node_ptr, grow_state);
    }

    /// grow the dual module and update [`DualModuleInterface::sum_`]
    pub fn grow(&mut self, length: Weight, dual_module_impl: &mut impl DualModuleImpl) {
        dual_module_impl.grow(length);
        self.sum_dual_variables += length * self.sum_grow_speed;
    }

}

impl Ord for MaxUpdateLength {
    fn cmp(&self, other: &Self) -> Ordering {
        debug_assert!(!matches!(self, MaxUpdateLength::NonZeroGrow(_)), "priority ordering is not valid for NonZeroGrow");
        debug_assert!(!matches!(other, MaxUpdateLength::NonZeroGrow(_)), "priority ordering is not valid for NonZeroGrow");
        if self == other {
            return Ordering::Equal
        }
        // VertexShrinkStop has the lowest priority: it should be put at the end of any ordered list
        // this is because solving VertexShrinkStop conflict is not possible, but when this happens, the primal module
        // should have put this node as a "-" node in the alternating tree, so there must be a parent and a child that
        // are "+" nodes, conflicting with each other at exactly this VertexShrinkStop node. In this case, as long as
        // one solves those "+" nodes conflicting, e.g. forming a blossom, this node's VertexShrinkStop conflict is automatically solved
        match (matches!(self, MaxUpdateLength::VertexShrinkStop( .. )), matches!(other, MaxUpdateLength::VertexShrinkStop( .. ))) {
            (true, false) => { return Ordering::Less },  // less priority
            (false, true) => { return Ordering::Greater },  // greater priority
            (true, true) => { return self.get_vertex_shrink_stop().unwrap().cmp(other.get_vertex_shrink_stop().unwrap()) },  // don't care, just compare pointer
            _ => { }
        }
        // then, blossom expanding has the low priority, because it's infrequent and expensive
        match (matches!(self, MaxUpdateLength::BlossomNeedExpand( .. )), matches!(other, MaxUpdateLength::BlossomNeedExpand( .. ))) {
            (true, false) => { return Ordering::Less },  // less priority
            (false, true) => { return Ordering::Greater },  // greater priority
            (true, true) => { return self.get_blossom_need_expand().unwrap().cmp(other.get_blossom_need_expand().unwrap()) },  // don't care, just compare pointer
            _ => { }
        }
        // We'll prefer match nodes internally instead of to boundary, because there might be less path connecting to boundary
        // this is only an attempt to optimize the MWPM decoder, but anyway it won't be an optimal decoder
        match (matches!(self, MaxUpdateLength::TouchingVirtual( .. )), matches!(other, MaxUpdateLength::TouchingVirtual( .. ))) {
            (true, false) => { return Ordering::Less },  // less priority
            (false, true) => { return Ordering::Greater },  // greater priority
            (true, true) => {
                let (a, c) = self.get_touching_virtual().unwrap();
                let (b, d) = other.get_touching_virtual().unwrap();
                return a.cmp(b).reverse().then(c.cmp(&d).reverse())
            },  // don't care, just compare pointer
            _ => { }
        }
        // last, both of them MUST be MaxUpdateLength::Conflicting
        let (a, c) = self.get_conflicting().unwrap();
        let (b, d) = other.get_conflicting().unwrap();
        a.cmp(b).reverse().then(c.cmp(&d).reverse())
    }
}

impl PartialOrd for MaxUpdateLength {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl MaxUpdateLength {

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

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_none_zero_growth(&self) -> Option<Weight> {
        match self {
            Self::NonZeroGrow(length) => { Some(*length) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_conflicting(&self) -> Option<(&DualNodePtr, &DualNodePtr)> {
        match self {
            Self::Conflicting(a, b) => { Some((a, b)) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_touching_virtual(&self) -> Option<(&DualNodePtr, VertexIndex)> {
        match self {
            Self::TouchingVirtual(a, b) => { Some((a, *b)) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_blossom_need_expand(&self) -> Option<&DualNodePtr> {
        match self {
            Self::BlossomNeedExpand(a) => { Some(a) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_vertex_shrink_stop(&self) -> Option<&DualNodePtr> {
        match self {
            Self::VertexShrinkStop(a) => { Some(a) },
            _ => { None },
        }
    }

}
