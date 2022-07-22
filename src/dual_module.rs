//! Dual Module
//! 
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

use super::util::*;
use std::sync::{Arc, Weak};
use crate::derivative::Derivative;
use crate::parking_lot::RwLock;
use core::cmp::Ordering;
use std::collections::BinaryHeap;
use super::visualize::*;
use std::collections::HashSet;


/// A dual node is either a blossom or a vertex
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum DualNodeClass {
    Blossom {
        nodes_circle: Vec<DualNodeWeak>,
        touching_children: Vec<(DualNodeWeak, DualNodeWeak)>,
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

impl DualNodeGrowState {

    pub fn is_against(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Grow, Self::Grow | Self::Stay) => true,
            (Self::Stay, Self::Grow) => true,
            _ => false,
        }
    }

}

/// gives the maximum absolute length to grow, if not possible, give the reason
#[derive(Derivative, PartialEq, Eq, Clone)]
#[derivative(Debug)]
pub enum MaxUpdateLength {
    /// non-zero maximum update length
    NonZeroGrow(Weight),
    /// conflicting growth
    Conflicting((DualNodeWeak, DualNodeWeak), (DualNodeWeak, DualNodeWeak)),  // (node_1, touching_1), (node_2, touching_2)
    /// conflicting growth because of touching virtual node
    TouchingVirtual((DualNodeWeak, DualNodeWeak), VertexIndex),  // (node, touching), virtual_vertex
    /// blossom hitting 0 dual variable while shrinking
    BlossomNeedExpand(DualNodeWeak),
    /// node hitting 0 dual variable while shrinking: note that this should have the lowest priority, normally it won't show up in a normal primal module
    VertexShrinkStop(DualNodeWeak),
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
    pub parent_blossom: Option<DualNodeWeak>,
    /// information used to compute dual variable of this node: (last dual variable, last global progress)
    dual_variable_cache: (Weight, Weight),
}

impl DualNode {

    /// get the current dual variable of a node
    pub fn get_dual_variable(&self, interface: &DualModuleInterface) -> Weight {
        let (last_dual_variable, last_global_progress) = self.dual_variable_cache;
        match self.grow_state {
            DualNodeGrowState::Grow => last_dual_variable + (interface.dual_variable_global_progress - last_global_progress),
            DualNodeGrowState::Stay => last_dual_variable,
            DualNodeGrowState::Shrink => last_dual_variable - (interface.dual_variable_global_progress - last_global_progress),
        }
    }

}

/// the shared pointer of [`DualNode`]
pub struct DualNodePtr { ptr: Arc<RwLock<DualNode>>, }
pub struct DualNodeWeak { ptr: Weak<RwLock<DualNode>>, }

impl DualNodePtr { pub fn downgrade(&self) -> DualNodeWeak { DualNodeWeak { ptr: Arc::downgrade(&self.ptr) } } }
impl DualNodeWeak { pub fn upgrade_force(&self) -> DualNodePtr { DualNodePtr { ptr: self.ptr.upgrade().unwrap() } } }

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

impl Clone for DualNodeWeak {
    fn clone(&self) -> Self {
       Self { ptr: self.ptr.clone() }
    }
}

impl std::fmt::Debug for DualNodeWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PartialEq for DualNodeWeak {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

impl Eq for DualNodeWeak { }

impl Ord for DualNodePtr {
    // a consistent compare (during a single program)
    fn cmp(&self, other: &Self) -> Ordering {
        if false {  // faster way: compare pointer address, just to have a consistent order between pointers
            let ptr1 = Arc::as_ptr(self.ptr());
            let ptr2 = Arc::as_ptr(other.ptr());
            // https://doc.rust-lang.org/reference/types/pointer.html
            // "When comparing raw pointers they are compared by their address, rather than by what they point to."
            ptr1.cmp(&ptr2)
        } else {
            let node1 = self.read_recursive();
            let node2 = other.read_recursive();
            node1.index.cmp(&node2.index)
        }
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

    /// get parent blossom recursively
    pub fn get_ancestor_blossom(&self) -> DualNodePtr {
        let dual_node = self.read_recursive();
        match &dual_node.parent_blossom {
            Some(ptr) => ptr.upgrade_force().get_ancestor_blossom(),
            None => self.clone(),
        }
    }

    /// get the parent blossom before the most parent one, useful when expanding a blossom
    pub fn get_secondary_ancestor_blossom(&self) -> DualNodePtr {
        let mut secondary_ancestor = self.clone();
        let mut ancestor = self.read_recursive().parent_blossom.as_ref().expect("secondary ancestor does not exist").upgrade_force();
        loop {
            let dual_node = ancestor.read_recursive();
            let new_ancestor = match &dual_node.parent_blossom {
                Some(weak) => weak.upgrade_force(),
                None => { return secondary_ancestor; },
            };
            drop(dual_node);
            secondary_ancestor = ancestor.clone();
            ancestor = new_ancestor;
        }
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
    /// information used to compute dual variable of this node: (last dual variable, last global progress)
    dual_variable_global_progress: Weight,
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

    /// remove a blossom, note that this dual node ptr is already expanded from the root: normally you only need to remove this blossom;
    /// when force flag is set, remove blossom even if its dual variable is not 0: this action cannot be undone
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

impl FusionVisualizer for DualModuleInterface {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        self.sanity_check().unwrap();
        let mut dual_nodes = Vec::<serde_json::Value>::new();
        for dual_node_ptr in self.nodes.iter() {
            if let Some(dual_node_ptr) = &dual_node_ptr {
                let dual_node = dual_node_ptr.read_recursive();
                dual_nodes.push(json!({
                    if abbrev { "o" } else { "blossom" }: match &dual_node.class {
                        DualNodeClass::Blossom { nodes_circle, .. } => Some(nodes_circle.iter().map(|node_ptr|
                            node_ptr.upgrade_force().read_recursive().index).collect::<Vec<NodeIndex>>()),
                        _ => None,
                    },
                    if abbrev { "t" } else { "touching_children" }: match &dual_node.class {
                        DualNodeClass::Blossom { touching_children, .. } => Some(touching_children.iter().map(|(node_ptr_1, node_ptr_2)|
                            (node_ptr_1.upgrade_force().read_recursive().index, node_ptr_2.upgrade_force().read_recursive().index)).collect::<Vec<(NodeIndex, NodeIndex)>>()),
                        _ => None,
                    },
                    if abbrev { "s" } else { "syndrome_vertex" }: match &dual_node.class {
                        DualNodeClass::SyndromeVertex { syndrome_index } => Some(syndrome_index),
                        _ => None,
                    },
                    if abbrev { "g" } else { "grow_state" }: match &dual_node.grow_state {
                        DualNodeGrowState::Grow => "grow",
                        DualNodeGrowState::Shrink => "shrink",
                        DualNodeGrowState::Stay => "stay",
                    },
                    if abbrev { "u" } else { "unit_growth" }: match &dual_node.grow_state {
                        DualNodeGrowState::Grow => 1,
                        DualNodeGrowState::Shrink => -1,
                        DualNodeGrowState::Stay => 0,
                    },
                    if abbrev { "p" } else { "parent_blossom" }: dual_node.parent_blossom.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
                }));
            } else {
                dual_nodes.push(json!(null));
            }
        }
        json!({
            "interface": {
                if abbrev { "s" } else { "sum_grow_speed" }: self.sum_grow_speed,
                if abbrev { "d" } else { "sum_dual_variables" }: self.sum_dual_variables,
            },
            "dual_nodes": dual_nodes,
        })
    }
}

impl DualModuleInterface {

    /// a dual module interface MUST be created given a concrete implementation of the dual module
    pub fn new(syndrome: &Vec<VertexIndex>, dual_module_impl: &mut impl DualModuleImpl) -> Self {
        let mut array = Self {
            nodes: Vec::new(),
            sum_grow_speed: 0,
            sum_dual_variables: 0,
            debug_print_actions: false,
            dual_variable_global_progress: 0,
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
            dual_variable_cache: (0, 0),
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
    pub fn create_blossom(&mut self, nodes_circle: Vec<DualNodePtr>, mut touching_children: Vec<(DualNodeWeak, DualNodeWeak)>
            , dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        if touching_children.len() == 0 {  // automatically fill the children, only works when nodes_circle consists of all syndrome nodes
            touching_children = nodes_circle.iter().map(|ptr| (ptr.downgrade(), ptr.downgrade())).collect();
        }
        assert_eq!(touching_children.len(), nodes_circle.len(), "circle length mismatch");
        let blossom_node_ptr = DualNodePtr::new(DualNode {
            index: self.nodes.len(),
            internal: None,
            class: DualNodeClass::Blossom {
                nodes_circle: vec![],
                touching_children: vec![],
            },
            grow_state: DualNodeGrowState::Grow,
            parent_blossom: None,
            dual_variable_cache: (0, self.dual_variable_global_progress),
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
            node.parent_blossom = Some(blossom_node_ptr.downgrade());
        }
        if self.debug_print_actions {
            eprintln!("[create blossom] {:?} -> {}", nodes_circle, self.nodes.len());
        }
        {  // fill in the nodes because they're in a valid state (all linked to this blossom)
            let mut node = blossom_node_ptr.write();
            node.index = self.nodes.len();
            node.class = DualNodeClass::Blossom {
                nodes_circle: nodes_circle.iter().map(|ptr| ptr.downgrade()).collect(),
                touching_children: touching_children,
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
            let node = blossom_node_ptr.read_recursive();
            if let DualNodeClass::Blossom { nodes_circle, .. } = &node.class {
                eprintln!("[expand blossom] {:?} -> {:?}", blossom_node_ptr, nodes_circle);
            } else { unreachable!() }
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
            DualNodeClass::Blossom { nodes_circle, .. } => {
                for node_weak in nodes_circle.iter() {
                    let node_ptr = node_weak.upgrade_force();
                    let mut node = node_ptr.write();
                    assert!(node.parent_blossom.is_some() && node.parent_blossom.as_ref().unwrap() == &blossom_node_ptr.downgrade()
                        , "internal error: parent blossom must be this blossom");
                    assert!(&node.grow_state == &DualNodeGrowState::Stay, "internal error: children node must be DualNodeGrowState::Stay");
                    node.parent_blossom = None;
                    drop(node);
                    {  // safest way: to avoid sub-optimal result being found, set all nodes to growing state
                        // WARNING: expanding a blossom like this way MAY CAUSE DEADLOCK!
                        // think about this extreme case: after a blossom is expanded, they may gradually form a new blossom and needs expanding again!
                        self.set_grow_state(&node_ptr, DualNodeGrowState::Grow, dual_module_impl);
                        // the solution is to provide two entry points, the two children of this blossom that directly connect to the two + node in the alternating tree
                        // only in that way it's guaranteed to make some progress without re-constructing this blossom
                        // It's the primal module's responsibility to avoid this happening, using the dual module's API: [``]
                    }
                }
            },
            _ => { unreachable!() }
        }
    }

    /// a helper function to update grow state
    pub fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState, dual_module_impl: &mut impl DualModuleImpl) {
        if self.debug_print_actions {
            eprintln!("[set grow state] {:?} {:?}", dual_node_ptr, grow_state);
        }
        {  // update sum_grow_speed and dual variable cache
            let mut node = dual_node_ptr.write();
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
            let current_dual_variable = node.get_dual_variable(self);
            node.dual_variable_cache = (current_dual_variable, self.dual_variable_global_progress);  // update the cache
        }
        dual_module_impl.set_grow_state(&dual_node_ptr, grow_state);  // call this before dual node actually sets; to give history information
        dual_node_ptr.set_grow_state(grow_state);
    }

    /// grow the dual module and update [`DualModuleInterface::sum_`]
    pub fn grow(&mut self, length: Weight, dual_module_impl: &mut impl DualModuleImpl) {
        dual_module_impl.grow(length);
        self.sum_dual_variables += length * self.sum_grow_speed;
        self.dual_variable_global_progress += length;
    }

    /// grow  a specific length globally but iteratively: will try to keep growing that much
    pub fn grow_iterative(&mut self, mut length: Weight, dual_module_impl: &mut impl DualModuleImpl) {
        while length > 0 {
            let max_update_length = dual_module_impl.compute_maximum_update_length();
            let safe_growth = max_update_length.get_none_zero_growth().expect(format!("iterative grow failed because of conflicts {max_update_length:?}").as_str());
            let growth = std::cmp::min(length, safe_growth);
            self.grow(growth, dual_module_impl);
            length -= growth;
        }
    }

    /// do a sanity check of if all the nodes are in consistent state
    pub fn sanity_check(&self) -> Result<(), String> {
        if false {
            eprintln!("[warning] sanity check disabled for dual_module.rs");
            return Ok(());
        }
        let mut visited_syndrome = HashSet::with_capacity(self.nodes.len() * 2);
        let mut sum_individual_dual_variable = 0;
        for (index, dual_node_ptr) in self.nodes.iter().enumerate() {
            match dual_node_ptr {
                Some(dual_node_ptr) => {
                    let dual_node = dual_node_ptr.read_recursive();
                    sum_individual_dual_variable += dual_node.get_dual_variable(self);
                    if dual_node.index != index { return Err(format!("dual node index wrong: expected {}, actual {}", index, dual_node.index)) }
                    if dual_node.internal.is_none() { return Err(format!("the dual node {} is not connected to an concrete implementation of dual module", dual_node.index)) }
                    match &dual_node.class {
                        DualNodeClass::Blossom { nodes_circle, touching_children } => {
                            for (idx, circle_node_weak) in nodes_circle.iter().enumerate() {
                                let circle_node_ptr = circle_node_weak.upgrade_force();
                                if &circle_node_ptr == dual_node_ptr { return Err(format!("a blossom should not contain itself")) }
                                let circle_node = circle_node_ptr.read_recursive();
                                if circle_node.parent_blossom.as_ref() != Some(&dual_node_ptr.downgrade()) {
                                    return Err(format!("blossom {} contains {} but child's parent pointer = {:?} is not pointing back"
                                        , dual_node.index, circle_node.index, circle_node.parent_blossom))
                                }
                                if circle_node.grow_state != DualNodeGrowState::Stay { return Err(format!("child node {} is not at Stay state", circle_node.index)) }
                                // check if circle node is still tracked, i.e. inside self.nodes
                                if circle_node.index >= self.nodes.len() || self.nodes[circle_node.index].is_none() {
                                    return Err(format!("child's index {} is not in the interface", circle_node.index))
                                }
                                let tracked_circle_node_ptr = self.nodes[circle_node.index].as_ref().unwrap();
                                if tracked_circle_node_ptr != &circle_node_ptr {
                                    return Err(format!("the tracked ptr of child {} is not what's being pointed", circle_node.index))
                                }
                                // check children belongings
                                let (child_weak_1, child_weak_2) = &touching_children[idx];
                                if matches!(circle_node.class, DualNodeClass::SyndromeVertex{..}) {
                                    if child_weak_1 != circle_node_weak { return Err(format!("touching child can only be syndrome node {}", circle_node.index)) }
                                    if child_weak_2 != circle_node_weak { return Err(format!("touching child can only be syndrome node {}", circle_node.index)) }
                                } else {
                                    let child_ptr_1 = child_weak_1.upgrade_force();
                                    let child_ptr_2 = child_weak_2.upgrade_force();
                                    let child_1_ancestor = child_ptr_1.get_ancestor_blossom();
                                    let child_2_ancestor = child_ptr_2.get_ancestor_blossom();
                                    let circle_ancestor = circle_node_ptr.get_ancestor_blossom();
                                    if child_1_ancestor != circle_ancestor { return Err(format!("{:?} is not descendent of {}", child_ptr_1, circle_node.index)) }
                                    if child_2_ancestor != circle_ancestor { return Err(format!("{:?} is not descendent of {}", child_ptr_2, circle_node.index)) }
                                }
                            }
                        },
                        DualNodeClass::SyndromeVertex { syndrome_index } => {
                            if visited_syndrome.contains(syndrome_index) { return Err(format!("duplicate syndrome index: {}", syndrome_index)) }
                            visited_syndrome.insert(*syndrome_index);
                        },
                    }
                    match &dual_node.parent_blossom {
                        Some(parent_blossom_weak) => {
                            if dual_node.grow_state != DualNodeGrowState::Stay { return Err(format!("child node {} is not at Stay state", dual_node.index)) }
                            let parent_blossom_ptr = parent_blossom_weak.upgrade_force();
                            let parent_blossom = parent_blossom_ptr.read_recursive();
                            // check if child is actually inside this blossom
                            match &parent_blossom.class {
                                DualNodeClass::Blossom { nodes_circle, .. } => {
                                    let mut found_match_count = 0;
                                    for node_weak in nodes_circle.iter() {
                                        let node_ptr = node_weak.upgrade_force();
                                        if &node_ptr == dual_node_ptr {
                                            found_match_count += 1;
                                        }
                                    }
                                    if found_match_count != 1 {
                                        return Err(format!("{} is the parent of {} but the child only presents {} times", parent_blossom.index, dual_node.index, found_match_count))
                                    }
                                }, _ => { return Err(format!("{}, as the parent of {}, is not a blossom", parent_blossom.index, dual_node.index)) }
                            }
                            // check if blossom is still tracked, i.e. inside self.nodes
                            if parent_blossom.index >= self.nodes.len() || self.nodes[parent_blossom.index].is_none() {
                                return Err(format!("parent blossom's index {} is not in the interface", parent_blossom.index))
                            }
                            let tracked_parent_blossom_ptr = self.nodes[parent_blossom.index].as_ref().unwrap();
                            if tracked_parent_blossom_ptr != &parent_blossom_ptr {
                                return Err(format!("the tracked ptr of parent blossom {} is not what's being pointed", parent_blossom.index))
                            }
                        }, _ => { }
                    }
                }, _ => { }
            }
        }
        if sum_individual_dual_variable != self.sum_dual_variables {
            return Err(format!("internal error: the sum of dual variables is {} but individual sum is {}", self.sum_dual_variables, sum_individual_dual_variable))
        }
        Ok(())
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
            (true, true) => { return self.get_vertex_shrink_stop().unwrap().cmp(&other.get_vertex_shrink_stop().unwrap()) },  // don't care, just compare pointer
            _ => { }
        }
        // then, blossom expanding has the low priority, because it's infrequent and expensive
        match (matches!(self, MaxUpdateLength::BlossomNeedExpand( .. )), matches!(other, MaxUpdateLength::BlossomNeedExpand( .. ))) {
            (true, false) => { return Ordering::Less },  // less priority
            (false, true) => { return Ordering::Greater },  // greater priority
            (true, true) => { return self.get_blossom_need_expand().unwrap().cmp(&other.get_blossom_need_expand().unwrap()) },  // don't care, just compare pointer
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
                return a.cmp(&b).reverse().then(c.cmp(&d).reverse())
            },  // don't care, just compare pointer
            _ => { }
        }
        // last, both of them MUST be MaxUpdateLength::Conflicting
        let (a, c) = self.get_conflicting().unwrap();
        let (b, d) = other.get_conflicting().unwrap();
        a.cmp(&b).reverse().then(c.cmp(&d).reverse())
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
        if let MaxUpdateLength::Conflicting((n1, _), (n2, _)) = self {
            if n1 == &a.downgrade() && n2 == &b.downgrade() {
                return true
            }
            if n1 == &b.downgrade() && n2 == &a.downgrade() {
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
    pub fn get_conflicting(&self) -> Option<(DualNodePtr, DualNodePtr)> {
        match self {
            Self::Conflicting((a, _), (b, _)) => { Some((a.upgrade_force(), b.upgrade_force())) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_touching_virtual(&self) -> Option<(DualNodePtr, VertexIndex)> {
        match self {
            Self::TouchingVirtual((a, _), b) => { Some((a.upgrade_force(), *b)) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_blossom_need_expand(&self) -> Option<DualNodePtr> {
        match self {
            Self::BlossomNeedExpand(a) => { Some(a.upgrade_force()) },
            _ => { None },
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_vertex_shrink_stop(&self) -> Option<DualNodePtr> {
        match self {
            Self::VertexShrinkStop(a) => { Some(a.upgrade_force()) },
            _ => { None },
        }
    }

}
