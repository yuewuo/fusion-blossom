//! Dual Module
//!
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

#![cfg_attr(feature = "unsafe_pointer", allow(dropping_references))]

use core::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::num::NonZeroUsize;
#[cfg(not(feature = "dangerous_pointer"))]
use std::sync::Arc;

use nonzero::nonzero as nz;

use crate::derivative::Derivative;

use super::pointers::*;
use super::util::*;
use super::visualize::*;

/// A dual node is either a blossom or a vertex
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum DualNodeClass {
    Blossom {
        nodes_circle: Vec<DualNodeWeak>,
        touching_children: Vec<(DualNodeWeak, DualNodeWeak)>,
    },
    DefectVertex {
        defect_index: VertexIndex,
    },
}

impl DualNodeClass {
    pub fn is_blossom(&self) -> bool {
        matches!(self, Self::Blossom { .. })
    }
}

/// Three possible states: Grow (+1), Stay (+0), Shrink (-1)
#[derive(Derivative, PartialEq, Eq, Clone, Copy)]
#[derivative(Debug)]
pub enum DualNodeGrowState {
    Grow,
    Stay,
    Shrink,
}

impl DualNodeGrowState {
    pub fn is_against(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Grow, Self::Grow | Self::Stay) | (Self::Stay, Self::Grow)
        )
    }
}

/// synchronize request on vertices, when a vertex is mirrored
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SyncRequest {
    /// the unit that owns this vertex
    pub mirror_unit_weak: PartitionUnitWeak,
    /// the vertex index to be synchronized
    pub vertex_index: VertexIndex,
    /// propagated dual node index and the dual variable of the propagated dual node;
    /// this field is necessary to differentiate between normal shrink and the one that needs to report VertexShrinkStop event, when the syndrome is on the interface;
    /// it also includes the representative vertex of the dual node, so that parents can keep track of whether it should be elevated
    pub propagated_dual_node: Option<(DualNodeWeak, Weight, VertexIndex)>,
    /// propagated grandson node: must be a syndrome node
    pub propagated_grandson_dual_node: Option<(DualNodeWeak, Weight, VertexIndex)>,
}

impl SyncRequest {
    /// update all the interface nodes to be up-to-date, only necessary when there are fusion
    pub fn update(&self) {
        if let Some((weak, ..)) = &self.propagated_dual_node {
            weak.upgrade_force().update();
        }
        if let Some((weak, ..)) = &self.propagated_grandson_dual_node {
            weak.upgrade_force().update();
        }
    }
}

/// gives the maximum absolute length to grow, if not possible, give the reason;
/// note that strong reference is stored in `MaxUpdateLength` so dropping these temporary messages are necessary to avoid memory leakage;
/// the strong reference is required when multiple `BlossomNeedExpand` event is reported in different partitions and sorting them requires a reference
#[derive(Derivative, PartialEq, Eq, Clone)]
#[derivative(Debug)]
pub enum MaxUpdateLength {
    /// non-zero maximum update length, has_empty_boundary_node (useful in fusion)
    NonZeroGrow((Weight, bool)),
    /// conflicting growth
    Conflicting((DualNodePtr, DualNodePtr), (DualNodePtr, DualNodePtr)), // (node_1, touching_1), (node_2, touching_2)
    /// conflicting growth because of touching virtual node
    TouchingVirtual((DualNodePtr, DualNodePtr), (VertexIndex, bool)), // (node, touching), (virtual_vertex, is_mirror)
    /// blossom hitting 0 dual variable while shrinking
    BlossomNeedExpand(DualNodePtr),
    /// node hitting 0 dual variable while shrinking: note that this should have the lowest priority, normally it won't show up in a normal primal module;
    /// in case that the dual module is partitioned and nobody can report this conflicting event, one needs to embed the potential conflicts using the second
    /// argument so that dual module can gather two `VertexShrinkStop` events to form a single `Conflicting` event
    VertexShrinkStop((DualNodePtr, Option<(DualNodePtr, DualNodePtr)>)),
}

cfg_if::cfg_if! {
    if #[cfg(feature="ordered_conflicts")] {
        use std::collections::BinaryHeap;
        pub type ConflictList = BinaryHeap<MaxUpdateLength>;
    } else {
        pub type ConflictList = Vec<MaxUpdateLength>;
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub enum GroupMaxUpdateLength {
    /// non-zero maximum update length, has_empty_boundary_node (useful in fusion)
    NonZeroGrow((Weight, bool)),
    /// conflicting reasons and pending VertexShrinkStop events (empty in a single serial dual module)
    Conflicts((ConflictList, BTreeMap<VertexIndex, MaxUpdateLength>)),
}

impl Default for GroupMaxUpdateLength {
    fn default() -> Self {
        Self::new()
    }
}

impl GroupMaxUpdateLength {
    pub fn new() -> Self {
        Self::NonZeroGrow((Weight::MAX, false))
    }

    /// update all the interface nodes to be up-to-date, only necessary in existence of fusion
    #[inline(never)]
    pub fn update(&self) {
        if let Self::Conflicts((list, pending_stops)) = &self {
            for max_update_length in list.iter() {
                max_update_length.update();
            }
            for (_, max_update_length) in pending_stops.iter() {
                max_update_length.update();
            }
        }
    }

    pub fn add_pending_stop(
        list: &mut ConflictList,
        pending_stops: &mut BTreeMap<VertexIndex, MaxUpdateLength>,
        max_update_length: MaxUpdateLength,
    ) {
        if let Some(dual_node_ptr) = max_update_length.get_vertex_shrink_stop() {
            let vertex_index = dual_node_ptr.get_representative_vertex();
            if let Some(existing_length) = pending_stops.get(&vertex_index) {
                if let MaxUpdateLength::VertexShrinkStop((_, Some(weak_pair))) = &max_update_length {
                    // otherwise don't update
                    if let MaxUpdateLength::VertexShrinkStop((_, Some(existing_weak_pair))) = existing_length {
                        if weak_pair.0 != existing_weak_pair.0 {
                            // two such conflicts form a Conflicting event
                            list.push(MaxUpdateLength::Conflicting(weak_pair.clone(), existing_weak_pair.clone()));
                            pending_stops.remove(&vertex_index);
                        }
                    } else {
                        pending_stops.insert(vertex_index, max_update_length.clone());
                        // update the entry
                    }
                }
            } else {
                pending_stops.insert(vertex_index, max_update_length);
            }
        } else {
            list.push(max_update_length);
        }
    }

    pub fn add(&mut self, max_update_length: MaxUpdateLength) {
        match self {
            Self::NonZeroGrow((current_length, current_has_empty_boundary_node)) => {
                if let MaxUpdateLength::NonZeroGrow((length, has_empty_boundary_node)) = max_update_length {
                    *current_length = std::cmp::min(*current_length, length);
                    *current_has_empty_boundary_node |= has_empty_boundary_node;
                // or
                } else {
                    let mut list = ConflictList::new();
                    let mut pending_stops = BTreeMap::new();
                    if let Some(dual_node_ptr) = max_update_length.get_vertex_shrink_stop() {
                        let vertex_index = dual_node_ptr.get_representative_vertex();
                        pending_stops.insert(vertex_index, max_update_length);
                    } else {
                        list.push(max_update_length);
                    }
                    *self = Self::Conflicts((list, pending_stops));
                }
            }
            Self::Conflicts((list, pending_stops)) => {
                // only add conflicts, not NonZeroGrow
                if !matches!(max_update_length, MaxUpdateLength::NonZeroGrow(_)) {
                    Self::add_pending_stop(list, pending_stops, max_update_length);
                }
            }
        }
    }

    pub fn extend(&mut self, other: Self) {
        if other.is_empty() {
            return; // do nothing
        }
        match self {
            Self::NonZeroGrow(current_length) => match other {
                Self::NonZeroGrow(length) => {
                    *current_length = std::cmp::min(*current_length, length);
                }
                Self::Conflicts((mut other_list, mut other_pending_stops)) => {
                    let mut list = ConflictList::new();
                    let mut pending_stops = BTreeMap::new();
                    std::mem::swap(&mut list, &mut other_list);
                    std::mem::swap(&mut pending_stops, &mut other_pending_stops);
                    *self = Self::Conflicts((list, pending_stops));
                }
            },
            Self::Conflicts((list, pending_stops)) => {
                if let Self::Conflicts((other_list, other_pending_stops)) = other {
                    list.extend(other_list);
                    for (_, max_update_length) in other_pending_stops.into_iter() {
                        Self::add_pending_stop(list, pending_stops, max_update_length);
                    }
                } // only add conflicts, not NonZeroGrow
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::NonZeroGrow((Weight::MAX, _))) // if `has_empty_boundary_node`, then it's not considered empty
    }

    pub fn is_active(&self) -> bool {
        !matches!(self, Self::NonZeroGrow((Weight::MAX, false))) // if `has_empty_boundary_node`, then it's still considered active
    }

    pub fn get_none_zero_growth(&self) -> Option<Weight> {
        match self {
            Self::NonZeroGrow((length, _has_empty_boundary_node)) => {
                debug_assert!(
                    *length != Weight::MAX,
                    "please call GroupMaxUpdateLength::is_empty to check if this group is empty"
                );
                Some(*length)
            }
            _ => None,
        }
    }

    pub fn pop(&mut self) -> Option<MaxUpdateLength> {
        match self {
            Self::NonZeroGrow(_) => {
                panic!("please call GroupMaxUpdateLength::get_none_zero_growth to check if this group is none_zero_growth");
            }
            Self::Conflicts((list, pending_stops)) => {
                list.pop().or(if let Some(key) = pending_stops.keys().next().cloned() {
                    pending_stops.remove(&key)
                } else {
                    None
                })
            }
        }
    }

    pub fn peek(&self) -> Option<&MaxUpdateLength> {
        match self {
            Self::NonZeroGrow(_) => {
                panic!("please call GroupMaxUpdateLength::get_none_zero_growth to check if this group is none_zero_growth");
            }
            Self::Conflicts((list, pending_stops)) => {
                cfg_if::cfg_if! {
                    if #[cfg(feature="ordered_conflicts")] {
                        let peek_element = list.peek();
                    } else {
                        let peek_element = list.last();
                    }
                }
                peek_element.or(if pending_stops.is_empty() {
                    None
                } else {
                    pending_stops.values().next()
                })
            }
        }
    }
}

/// A dual node corresponds to either a vertex or a blossom (on which the dual variables are defined)
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct DualNode {
    /// the index of this dual node, helps to locate internal details of this dual node
    pub index: NodeIndex,
    /// the class of this dual node
    pub class: DualNodeClass,
    /// whether it grows, stays or shrinks
    pub grow_state: DualNodeGrowState,
    /// parent blossom: when parent exists, grow_state should be [`DualNodeGrowState::Stay`]
    pub parent_blossom: Option<DualNodeWeak>,
    /// information used to compute dual variable of this node: (last dual variable, last global progress)
    pub dual_variable_cache: (Weight, Weight),
    /// belonging of the dual module interface; a dual node is never standalone
    pub belonging: DualModuleInterfaceWeak,
    /// how many defect vertices in this dual node
    pub defect_size: NonZeroUsize,
}

impl DualNode {
    /// get the current dual variable of a node
    pub fn get_dual_variable(&self, interface: &DualModuleInterface) -> Weight {
        let (last_dual_variable, last_global_progress) = self.dual_variable_cache;
        match self.grow_state {
            DualNodeGrowState::Grow => last_dual_variable + (interface.dual_variable_global_progress - last_global_progress),
            DualNodeGrowState::Stay => last_dual_variable,
            DualNodeGrowState::Shrink => {
                last_dual_variable - (interface.dual_variable_global_progress - last_global_progress)
            }
        }
    }
}

// should not use dangerous pointer because expanding a blossom will leave a weak pointer invalid
pub type DualNodePtr = ArcManualSafeLock<DualNode>;
pub type DualNodeWeak = WeakManualSafeLock<DualNode>;

impl Ord for DualNodePtr {
    // a consistent compare (during a single program)
    fn cmp(&self, other: &Self) -> Ordering {
        cfg_if::cfg_if! {
            if #[cfg(feature="dangerous_pointer")] {
                let node1 = self.read_recursive();
                let node2 = other.read_recursive();
                node1.index.cmp(&node2.index)
            } else {
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
    }
}

impl PartialOrd for DualNodePtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Debug for DualNodePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.update(); // to make sure index is up-to-date
        let dual_node = self.read_recursive(); // reading index is consistent
        write!(f, "{}", dual_node.index)
    }
}

impl std::fmt::Debug for DualNodeWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl DualNodePtr {
    /// when fused, dual node may be outdated; refresh here
    #[allow(clippy::needless_borrow)]
    pub fn update(&self) -> &Self {
        let mut current_belonging = self.read_recursive().belonging.upgrade_force();
        let mut bias = 0;
        let mut node = self.write();
        while current_belonging.read_recursive().parent.is_some() {
            let belonging_interface = current_belonging.read_recursive();
            bias += belonging_interface.index_bias;
            let new_current_belonging = belonging_interface.parent.clone().unwrap().upgrade_force();
            let dual_variable = node.get_dual_variable(&belonging_interface); // aggregate the dual variable
            node.dual_variable_cache = (dual_variable, 0); // this will be the state when joining the new interface
            drop(belonging_interface);
            current_belonging = new_current_belonging;
        }
        node.belonging = current_belonging.downgrade();
        node.index += bias;
        self
    }

    pub fn updated_index(&self) -> NodeIndex {
        self.update();
        self.read_recursive().index
    }

    /// helper function to set grow state with sanity check
    fn set_grow_state(&self, grow_state: DualNodeGrowState) {
        let mut dual_node = self.write();
        debug_assert!(
            dual_node.parent_blossom.is_none(),
            "setting node grow state inside a blossom forbidden"
        );
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
        let mut ancestor = self
            .read_recursive()
            .parent_blossom
            .as_ref()
            .expect("secondary ancestor does not exist")
            .upgrade_force();
        loop {
            let dual_node = ancestor.read_recursive();
            let new_ancestor = match &dual_node.parent_blossom {
                Some(weak) => weak.upgrade_force(),
                None => {
                    return secondary_ancestor;
                }
            };
            drop(dual_node);
            secondary_ancestor = ancestor.clone();
            ancestor = new_ancestor;
        }
    }

    fn __get_all_vertices(&self, pending_vec: &mut Vec<VertexIndex>) {
        let dual_node = self.read_recursive();
        match &dual_node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => {
                for node_ptr in nodes_circle.iter() {
                    node_ptr.upgrade_force().__get_all_vertices(pending_vec);
                }
            }
            DualNodeClass::DefectVertex { defect_index } => {
                pending_vec.push(*defect_index);
            }
        };
    }

    /// find all vertices that belongs to the dual node, i.e. any vertices inside a blossom
    pub fn get_all_vertices(&self) -> Vec<VertexIndex> {
        let mut pending_vec = vec![];
        self.__get_all_vertices(&mut pending_vec);
        pending_vec
    }

    /// find a representative vertex
    pub fn get_representative_vertex(&self) -> VertexIndex {
        let dual_node = self.read_recursive();
        match &dual_node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => nodes_circle[0].upgrade_force().get_representative_vertex(),
            DualNodeClass::DefectVertex { defect_index } => *defect_index,
        }
    }
}

/// a sharable array of dual nodes, supporting dynamic partitioning;
/// note that a node can be destructed and we do not reuse its index, leaving a blank space
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualModuleInterface {
    /// unit index of this interface, default to 0
    pub unit_index: usize,
    /// all the dual node that can be used to control a concrete dual module implementation
    pub nodes: Vec<Option<DualNodePtr>>,
    /// current nodes length, to enable constant-time clear operation
    pub nodes_length: usize,
    /// allow pointer reuse will reduce the time of reallocation, but it's unsafe if not owning it;
    /// this will be automatically disabled when [`DualModuleInterface::fuse`] is called;
    /// if an interface is involved in a fusion operation (whether as parent or child), it will be set.
    pub is_fusion: bool,
    /// record the total growing nodes, should be non-negative in a normal running algorithm
    pub sum_grow_speed: Weight,
    /// record the total sum of dual variables
    pub sum_dual_variables: Weight,
    /// debug mode: only resolve one conflict each time
    pub debug_print_actions: bool,
    /// information used to compute dual variable of this node: (last dual variable, last global progress)
    dual_variable_global_progress: Weight,
    /// the parent of this interface, when fused
    pub parent: Option<DualModuleInterfaceWeak>,
    /// when fused, this will indicate the relative bias given by the parent
    pub index_bias: NodeIndex,
    /// the two children of this interface, when fused; following the length of this child,
    /// given that fused children interface will not have new nodes anymore
    pub children: Option<((DualModuleInterfaceWeak, NodeIndex), (DualModuleInterfaceWeak, NodeIndex))>,
}

pub type DualModuleInterfacePtr = ArcManualSafeLock<DualModuleInterface>;
pub type DualModuleInterfaceWeak = WeakManualSafeLock<DualModuleInterface>;

impl std::fmt::Debug for DualModuleInterfacePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let interface = self.read_recursive();
        write!(f, "{}", interface.unit_index)
    }
}

impl std::fmt::Debug for DualModuleInterfaceWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

/// common trait that must be implemented for each implementation of dual module
pub trait DualModuleImpl {
    /// create a new dual module with empty syndrome
    fn new_empty(initializer: &SolverInitializer) -> Self;

    /// clear all growth and existing dual nodes, prepared for the next decoding
    fn clear(&mut self);

    /// add corresponding dual node
    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr);

    #[inline(always)]
    /// helper function to specifically add a syndrome node
    fn add_defect_node(&mut self, dual_node_ptr: &DualNodePtr) {
        debug_assert!(
            {
                let node = dual_node_ptr.read_recursive();
                matches!(node.class, DualNodeClass::DefectVertex { .. })
            },
            "node class mismatch"
        );
        self.add_dual_node(dual_node_ptr)
    }

    #[inline(always)]
    /// helper function to specifically add a blossom node
    fn add_blossom(&mut self, dual_node_ptr: &DualNodePtr) {
        debug_assert!(
            {
                let node = dual_node_ptr.read_recursive();
                matches!(node.class, DualNodeClass::Blossom { .. })
            },
            "node class mismatch"
        );
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
    fn compute_maximum_update_length_dual_node(
        &mut self,
        _dual_node_ptr: &DualNodePtr,
        _is_grow: bool,
        _simultaneous_update: bool,
    ) -> MaxUpdateLength {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// check the maximum length to grow (shrink) for all nodes, return a list of conflicting reason and a single number indicating the maximum length to grow:
    /// this number will be 0 if any conflicting reason presents
    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength;

    /// An optional function that can manipulate individual dual node, not necessarily supported by all implementations
    fn grow_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _length: Weight) {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// grow a specific length globally, length must be positive.
    /// note that reversing the process is possible, but not recommended: to do that, reverse the state of each dual node, Grow->Shrink, Shrink->Grow
    fn grow(&mut self, length: Weight);

    /// optional support for edge modifier. for example, erasure errors temporarily set some edges to 0 weight.
    /// When it clears, those edges must be reverted back to the original weight
    fn load_edge_modifier(&mut self, _edge_modifier: &[(EdgeIndex, Weight)]) {
        unimplemented!(
            "load_edge_modifier is an optional interface, and the current dual module implementation doesn't support it"
        );
    }

    /// an erasure error means this edge is totally uncertain: p=0.5, so new weight = ln((1-p)/p) = 0
    fn load_erasures(&mut self, erasures: &[EdgeIndex]) {
        let edge_modifier: Vec<_> = erasures.iter().map(|edge_index| (*edge_index, 0)).collect();
        self.load_edge_modifier(&edge_modifier);
    }

    fn load_dynamic_weights(&mut self, dynamic_weights: &[(EdgeIndex, Weight)]) {
        let edge_modifier = dynamic_weights.to_vec();
        self.load_edge_modifier(&edge_modifier);
    }

    /// prepare a list of nodes as shrinking state; useful in creating a blossom
    fn prepare_nodes_shrink(&mut self, _nodes_circle: &[DualNodePtr]) -> &mut Vec<SyncRequest> {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// performance profiler report
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }

    /*
     * the following apis are only required when this dual module can be used as a partitioned one
     */

    /// create a partitioned dual module (hosting only a subgraph and subset of dual nodes) to be used in the parallel dual module
    fn new_partitioned(_partitioned_initializer: &PartitionedSolverInitializer) -> Self
    where
        Self: std::marker::Sized,
    {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// prepare the growing or shrinking state of all nodes and return a list of sync requests in case of mirrored vertices are changed
    fn prepare_all(&mut self) -> &mut Vec<SyncRequest> {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// execute a synchronize event by updating the state of a vertex and also update the internal dual node accordingly
    fn execute_sync_event(&mut self, _sync_event: &SyncRequest) {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// judge whether the current module hosts the dual node
    fn contains_dual_node(&self, _dual_node_ptr: &DualNodePtr) -> bool {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// judge whether the current module hosts any of these dual node
    fn contains_dual_nodes_any(&self, dual_node_ptrs: &[DualNodePtr]) -> bool {
        for dual_node_ptr in dual_node_ptrs.iter() {
            if self.contains_dual_node(dual_node_ptr) {
                return true;
            }
        }
        false
    }

    /// judge whether the current module hosts a vertex
    fn contains_vertex(&self, _vertex_index: VertexIndex) -> bool {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }

    /// bias the global dual node indices
    fn bias_dual_node_index(&mut self, _bias: NodeIndex) {
        panic!("the dual module implementation doesn't support this function, please use another dual module")
    }
}

/// this dual module is a parallel version that hosts many partitioned ones
pub trait DualModuleParallelImpl {
    type UnitType: DualModuleImpl + Send + Sync;

    fn get_unit(&self, unit_index: usize) -> ArcManualSafeLock<Self::UnitType>;
}

impl FusionVisualizer for DualModuleInterfacePtr {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        let flattened_nodes = self.sanity_check().unwrap();
        let interface = self.read_recursive();
        let mut dual_nodes = Vec::<serde_json::Value>::new();
        for dual_node_ptr in flattened_nodes.iter() {
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
                    if abbrev { "s" } else { "defect_vertex" }: match &dual_node.class {
                        DualNodeClass::DefectVertex { defect_index } => Some(defect_index),
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
                if abbrev { "s" } else { "sum_grow_speed" }: interface.sum_grow_speed,
                if abbrev { "d" } else { "sum_dual_variables" }: interface.sum_dual_variables,
            },
            "dual_nodes": dual_nodes,
        })
    }
}

impl DualModuleInterface {
    /// return the count of all nodes including those of the children interfaces
    pub fn nodes_count(&self) -> NodeNum {
        let mut count = self.nodes_length as NodeNum;
        if let Some(((_, left_count), (_, right_count))) = &self.children {
            count += left_count + right_count;
        }
        count
    }

    /// get node ptr by index; if calling from the ancestor interface, node_index is absolute, otherwise it's relative
    #[allow(clippy::unnecessary_cast)]
    pub fn get_node(&self, relative_node_index: NodeIndex) -> Option<DualNodePtr> {
        debug_assert!(relative_node_index < self.nodes_count(), "cannot find node in this interface");
        let mut bias = 0;
        if let Some(((left_weak, left_count), (right_weak, right_count))) = &self.children {
            if relative_node_index < *left_count {
                // this node belongs to the left
                return left_weak.upgrade_force().read_recursive().get_node(relative_node_index);
            } else if relative_node_index < *left_count + *right_count {
                // this node belongs to the right
                return right_weak
                    .upgrade_force()
                    .read_recursive()
                    .get_node(relative_node_index - *left_count);
            }
            bias = left_count + right_count;
        }
        self.nodes[(relative_node_index - bias) as usize].clone()
    }

    /// set the corresponding node index to None
    #[allow(clippy::unnecessary_cast)]
    pub fn remove_node(&mut self, relative_node_index: NodeIndex) {
        debug_assert!(relative_node_index < self.nodes_count(), "cannot find node in this interface");
        let mut bias = 0;
        if let Some(((left_weak, left_count), (right_weak, right_count))) = &self.children {
            if relative_node_index < *left_count {
                // this node belongs to the left
                left_weak.upgrade_force().write().remove_node(relative_node_index);
                return;
            } else if relative_node_index < *left_count + *right_count {
                // this node belongs to the right
                right_weak
                    .upgrade_force()
                    .write()
                    .remove_node(relative_node_index - *left_count);
                return;
            }
            bias = left_count + right_count;
        }
        self.nodes[(relative_node_index - bias) as usize] = None;
    }
}

impl DualModuleInterfacePtr {
    /// create an empty interface
    pub fn new_empty() -> Self {
        Self::new_value(DualModuleInterface {
            unit_index: 0, // if necessary, manually change it
            nodes: Vec::new(),
            nodes_length: 0,
            is_fusion: false,
            sum_grow_speed: 0,
            sum_dual_variables: 0,
            debug_print_actions: false,
            dual_variable_global_progress: 0,
            parent: None,
            index_bias: 0,
            children: None,
        })
    }

    /// a dual module interface MUST be created given a concrete implementation of the dual module
    pub fn new_load(syndrome_pattern: &SyndromePattern, dual_module_impl: &mut impl DualModuleImpl) -> Self {
        let interface_ptr = Self::new_empty();
        interface_ptr.load(syndrome_pattern, dual_module_impl);
        interface_ptr
    }

    pub fn load(&self, syndrome_pattern: &SyndromePattern, dual_module_impl: &mut impl DualModuleImpl) {
        for vertex_idx in syndrome_pattern.defect_vertices.iter() {
            self.create_defect_node(*vertex_idx, dual_module_impl);
        }
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            dual_module_impl.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            dual_module_impl.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
    }

    /// a constant clear function, without dropping anything;
    /// this is for consideration of reducing the garbage collection time in the parallel solver,
    /// by distributing the clear cost into each thread but not the single main thread.
    pub fn clear(&self) {
        let mut interface = self.write();
        interface.nodes_length = 0;
        interface.sum_grow_speed = 0;
        interface.sum_dual_variables = 0;
        interface.dual_variable_global_progress = 0;
        interface.is_fusion = false;
        interface.parent = None;
        interface.index_bias = 0;
        interface.children = None;
    }

    /// DFS flatten the nodes
    pub fn flatten_nodes(&self, flattened_nodes: &mut Vec<Option<DualNodePtr>>) {
        let interface = self.read_recursive();
        let flattened_nodes_length = flattened_nodes.len() as NodeNum;
        // the order matters: left -> right -> myself
        if let Some(((left_child_weak, _), (right_child_weak, _))) = &interface.children {
            left_child_weak.upgrade_force().flatten_nodes(flattened_nodes);
            right_child_weak.upgrade_force().flatten_nodes(flattened_nodes);
        }
        for node_index in 0..interface.nodes_length {
            let dual_node_ptr = &interface.nodes[node_index];
            if let Some(dual_node_ptr) = dual_node_ptr {
                dual_node_ptr.update();
            }
            flattened_nodes.push(dual_node_ptr.clone());
        }
        debug_assert_eq!(
            flattened_nodes.len() as NodeNum - flattened_nodes_length,
            interface.nodes_count()
        );
    }

    pub fn create_defect_node(&self, vertex_idx: VertexIndex, dual_module_impl: &mut impl DualModuleImpl) -> DualNodePtr {
        let belonging = self.downgrade();
        let mut interface = self.write();
        interface.sum_grow_speed += 1;
        let local_node_index = interface.nodes_length;
        let node_index = interface.nodes_count();
        // try to reuse existing pointer to avoid list allocation
        let node_ptr = if !interface.is_fusion
            && local_node_index < interface.nodes.len()
            && interface.nodes[local_node_index].is_some()
        {
            let node_ptr = interface.nodes[local_node_index].take().unwrap();
            let mut node = node_ptr.write();
            node.index = node_index;
            node.class = DualNodeClass::DefectVertex {
                defect_index: vertex_idx,
            };
            node.grow_state = DualNodeGrowState::Grow;
            node.parent_blossom = None;
            node.dual_variable_cache = (0, interface.dual_variable_global_progress);
            node.belonging = belonging;
            node.defect_size = nz!(1usize);
            drop(node);
            node_ptr
        } else {
            DualNodePtr::new_value(DualNode {
                index: node_index,
                class: DualNodeClass::DefectVertex {
                    defect_index: vertex_idx,
                },
                grow_state: DualNodeGrowState::Grow,
                parent_blossom: None,
                dual_variable_cache: (0, interface.dual_variable_global_progress),
                belonging,
                defect_size: nz!(1usize),
            })
        };
        interface.nodes_length += 1;
        if interface.nodes.len() < interface.nodes_length {
            interface.nodes.push(None);
        }
        let cloned_node_ptr = node_ptr.clone();
        interface.nodes[local_node_index] = Some(node_ptr); // feature `dangerous_pointer`: must push the owner
        drop(interface);
        dual_module_impl.add_defect_node(&cloned_node_ptr);
        cloned_node_ptr
    }

    /// check whether a pointer belongs to this node, it will acquire a reader lock on `dual_node_ptr`
    pub fn check_ptr_belonging(&self, dual_node_ptr: &DualNodePtr) -> bool {
        let interface = self.read_recursive();
        let dual_node = dual_node_ptr.read_recursive();
        if dual_node.index >= interface.nodes_count() {
            return false;
        }
        if let Some(ptr) = interface.get_node(dual_node.index).as_ref() {
            ptr == dual_node_ptr
        } else {
            false
        }
    }

    /// create a dual node corresponding to a blossom, automatically set the grow state of internal nodes;
    /// the nodes circle MUST starts with a growing node and ends with a shrinking node
    pub fn create_blossom(
        &self,
        nodes_circle: Vec<DualNodePtr>,
        mut touching_children: Vec<(DualNodeWeak, DualNodeWeak)>,
        dual_module_impl: &mut impl DualModuleImpl,
    ) -> DualNodePtr {
        let belonging = self.downgrade();
        let mut interface = self.write();
        if touching_children.is_empty() {
            // automatically fill the children, only works when nodes_circle consists of all syndrome nodes
            touching_children = nodes_circle.iter().map(|ptr| (ptr.downgrade(), ptr.downgrade())).collect();
        }
        debug_assert_eq!(touching_children.len(), nodes_circle.len(), "circle length mismatch");
        let local_node_index = interface.nodes_length;
        let node_index = interface.nodes_count();
        let defect_size = nodes_circle
            .iter()
            .map(|iter| iter.read_recursive().defect_size)
            .reduce(|a, b| a.saturating_add(b.get()))
            .unwrap();

        let blossom_node_ptr = if !interface.is_fusion
            && local_node_index < interface.nodes.len()
            && interface.nodes[local_node_index].is_some()
        {
            let node_ptr = interface.nodes[local_node_index].take().unwrap();
            let mut node = node_ptr.write();
            node.index = node_index;
            node.class = DualNodeClass::Blossom {
                nodes_circle: vec![],
                touching_children: vec![],
            };
            node.grow_state = DualNodeGrowState::Grow;
            node.parent_blossom = None;
            node.dual_variable_cache = (0, interface.dual_variable_global_progress);
            node.belonging = belonging;
            node.defect_size = defect_size;
            drop(node);
            node_ptr
        } else {
            DualNodePtr::new_value(DualNode {
                index: node_index,
                class: DualNodeClass::Blossom {
                    nodes_circle: vec![],
                    touching_children: vec![],
                },
                grow_state: DualNodeGrowState::Grow,
                parent_blossom: None,
                dual_variable_cache: (0, interface.dual_variable_global_progress),
                belonging,
                defect_size,
            })
        };
        drop(interface);
        for node_ptr in nodes_circle.iter() {
            debug_assert!(
                self.check_ptr_belonging(node_ptr),
                "this ptr doesn't belong to this interface"
            );
            let node = node_ptr.read_recursive();
            debug_assert!(
                node.parent_blossom.is_none(),
                "cannot create blossom on a node that already belongs to a blossom"
            );
            drop(node);
            // set state must happen before setting parent
            self.set_grow_state(node_ptr, DualNodeGrowState::Stay, dual_module_impl);
            // then update parent
            let mut node = node_ptr.write();
            node.parent_blossom = Some(blossom_node_ptr.downgrade());
        }
        let mut interface = self.write();
        if interface.debug_print_actions {
            eprintln!("[create blossom] {:?} -> {}", nodes_circle, node_index);
        }
        let cloned_blossom_node_ptr = blossom_node_ptr.clone();
        {
            // fill in the nodes because they're in a valid state (all linked to this blossom)
            let mut node = blossom_node_ptr.write();
            node.class = DualNodeClass::Blossom {
                nodes_circle: nodes_circle.iter().map(|ptr| ptr.downgrade()).collect(),
                touching_children,
            };
            interface.nodes_length += 1;
            if interface.nodes.len() < interface.nodes_length {
                interface.nodes.push(None);
            }
            drop(node);
            interface.nodes[local_node_index] = Some(blossom_node_ptr); // feature `dangerous_pointer`: must push the owner
        }
        interface.sum_grow_speed += 1;
        drop(interface);
        dual_module_impl.prepare_nodes_shrink(&nodes_circle);
        dual_module_impl.add_blossom(&cloned_blossom_node_ptr);
        cloned_blossom_node_ptr
    }

    /// expand a blossom: note that different from Blossom V library, we do not maintain tree structure after a blossom is expanded;
    /// this is because we're growing all trees together, and due to the natural of quantum codes, this operation is not likely to cause
    /// bottleneck as long as physical error rate is well below the threshold. All internal nodes will have a [`DualNodeGrowState::Grow`] state afterwards.
    pub fn expand_blossom(&self, blossom_node_ptr: DualNodePtr, dual_module_impl: &mut impl DualModuleImpl) {
        let interface = self.read_recursive();
        if interface.debug_print_actions {
            let node = blossom_node_ptr.read_recursive();
            if let DualNodeClass::Blossom { nodes_circle, .. } = &node.class {
                eprintln!("[expand blossom] {:?} -> {:?}", blossom_node_ptr, nodes_circle);
            } else {
                unreachable!()
            }
        }
        let is_fusion = interface.is_fusion;
        drop(interface);
        if is_fusion {
            // must update all the nodes before calling `remove_blossom` of the implementation
            let node = blossom_node_ptr.read_recursive();
            if let DualNodeClass::Blossom { nodes_circle, .. } = &node.class {
                for node_weak in nodes_circle.iter() {
                    node_weak.upgrade_force().update();
                }
            }
        }
        dual_module_impl.remove_blossom(blossom_node_ptr.clone());
        let mut interface = self.write();
        let node = blossom_node_ptr.read_recursive();
        match &node.grow_state {
            DualNodeGrowState::Grow => {
                interface.sum_grow_speed += -1;
            }
            DualNodeGrowState::Shrink => {
                interface.sum_grow_speed += 1;
            }
            DualNodeGrowState::Stay => {}
        }
        let node_idx = node.index;
        debug_assert!(
            interface.get_node(node_idx).is_some(),
            "the blossom should not be expanded before"
        );
        debug_assert!(
            interface.get_node(node_idx).as_ref().unwrap() == &blossom_node_ptr,
            "the blossom doesn't belong to this DualModuleInterface"
        );
        drop(interface);
        match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => {
                for node_weak in nodes_circle.iter() {
                    let node_ptr = node_weak.upgrade_force();
                    let mut node = node_ptr.write();
                    debug_assert!(
                        node.parent_blossom.is_some()
                            && node.parent_blossom.as_ref().unwrap() == &blossom_node_ptr.downgrade(),
                        "internal error: parent blossom must be this blossom"
                    );
                    debug_assert!(
                        node.grow_state == DualNodeGrowState::Stay,
                        "internal error: children node must be DualNodeGrowState::Stay"
                    );
                    node.parent_blossom = None;
                    drop(node);
                    {
                        // safest way: to avoid sub-optimal result being found, set all nodes to growing state
                        // WARNING: expanding a blossom like this way MAY CAUSE DEADLOCK!
                        // think about this extreme case: after a blossom is expanded, they may gradually form a new blossom and needs expanding again!
                        self.set_grow_state(&node_ptr, DualNodeGrowState::Grow, dual_module_impl);
                        // the solution is to provide two entry points, the two children of this blossom that directly connect to the two + node in the alternating tree
                        // only in that way it's guaranteed to make some progress without re-constructing this blossom
                        // It's the primal module's responsibility to avoid this happening, using the dual module's API: [``]
                    }
                }
            }
            _ => {
                unreachable!()
            }
        }
        let mut interface = self.write();
        interface.remove_node(node_idx); // remove this blossom from root, feature `dangerous_pointer` requires running this at the end
    }

    /// a helper function to update grow state
    #[allow(clippy::needless_borrow)]
    pub fn set_grow_state(
        &self,
        dual_node_ptr: &DualNodePtr,
        grow_state: DualNodeGrowState,
        dual_module_impl: &mut impl DualModuleImpl,
    ) {
        if self.read_recursive().is_fusion {
            dual_node_ptr.update(); // these dual node may not be update-to-date in fusion
        }
        let mut interface = self.write();
        if interface.debug_print_actions {
            eprintln!("[set grow state] {:?} {:?}", dual_node_ptr, grow_state);
        }
        {
            // update sum_grow_speed and dual variable cache
            let mut node = dual_node_ptr.write();
            match &node.grow_state {
                DualNodeGrowState::Grow => {
                    interface.sum_grow_speed -= 1;
                }
                DualNodeGrowState::Shrink => {
                    interface.sum_grow_speed += 1;
                }
                DualNodeGrowState::Stay => {}
            }
            match grow_state {
                DualNodeGrowState::Grow => {
                    interface.sum_grow_speed += 1;
                }
                DualNodeGrowState::Shrink => {
                    interface.sum_grow_speed -= 1;
                }
                DualNodeGrowState::Stay => {}
            }
            let current_dual_variable = node.get_dual_variable(&interface);
            node.dual_variable_cache = (current_dual_variable, interface.dual_variable_global_progress);
            // update the cache
        }
        drop(interface);
        dual_module_impl.set_grow_state(dual_node_ptr, grow_state); // call this before dual node actually sets; to give history information
        dual_node_ptr.set_grow_state(grow_state);
    }

    /// grow the dual module and update [`DualModuleInterface::sum_`]
    pub fn grow(&self, length: Weight, dual_module_impl: &mut impl DualModuleImpl) {
        dual_module_impl.grow(length);
        self.notify_grown(length);
    }

    /// if a dual module spontaneously grow some value (e.g. with primal offloading), this function should be called
    pub fn notify_grown(&self, length: Weight) {
        let mut interface = self.write();
        interface.sum_dual_variables += length * interface.sum_grow_speed;
        interface.dual_variable_global_progress += length;
    }

    /// grow a specific length globally but iteratively: will try to keep growing that much
    pub fn grow_iterative(&self, mut length: Weight, dual_module_impl: &mut impl DualModuleImpl) {
        while length > 0 {
            let max_update_length = dual_module_impl.compute_maximum_update_length();
            let safe_growth = max_update_length
                .get_none_zero_growth()
                .unwrap_or_else(|| panic!("iterative grow failed because of conflicts {max_update_length:?}"));
            let growth = std::cmp::min(length, safe_growth);
            self.grow(growth, dual_module_impl);
            length -= growth;
        }
    }

    /// fuse two interfaces by copying the nodes in `other` into myself
    #[allow(clippy::unnecessary_cast)]
    #[allow(clippy::needless_borrow)]
    pub fn slow_fuse(&self, left: &Self, right: &Self) {
        let mut interface = self.write();
        interface.is_fusion = true; // for safety
        for other in [left, right] {
            let mut other_interface = other.write();
            other_interface.is_fusion = true;
            let bias = interface.nodes_length as NodeNum;
            for other_node_index in 0..other_interface.nodes_length as NodeNum {
                let node_ptr = &other_interface.nodes[other_node_index as usize];
                if let Some(node_ptr) = node_ptr {
                    let mut node = node_ptr.write();
                    debug_assert_eq!(node.index, other_node_index);
                    node.index += bias;
                    node.dual_variable_cache = (
                        node.get_dual_variable(&other_interface),
                        interface.dual_variable_global_progress,
                    )
                }
                interface.nodes_length += 1;
                if interface.nodes.len() < interface.nodes_length {
                    interface.nodes.push(None);
                }
                interface.nodes[(bias + other_node_index) as usize] = node_ptr.clone();
            }
            interface.sum_dual_variables += other_interface.sum_dual_variables;
            interface.sum_grow_speed += other_interface.sum_grow_speed;
        }
    }

    /// fuse two interfaces by (virtually) copying the nodes in `other` into myself, with O(1) time complexity
    pub fn fuse(&self, left: &Self, right: &Self) {
        let parent_weak = self.downgrade();
        let left_weak = left.downgrade();
        let right_weak = right.downgrade();
        let mut interface = self.write();
        interface.is_fusion = true; // for safety
        debug_assert_eq!(interface.nodes_length, 0, "fast fuse doesn't support non-empty fuse");
        debug_assert!(interface.children.is_none(), "cannot fuse twice");
        let mut left_interface = left.write();
        let mut right_interface = right.write();
        left_interface.is_fusion = true;
        right_interface.is_fusion = true;
        debug_assert!(left_interface.parent.is_none(), "cannot fuse an interface twice");
        debug_assert!(right_interface.parent.is_none(), "cannot fuse an interface twice");
        left_interface.parent = Some(parent_weak.clone());
        right_interface.parent = Some(parent_weak);
        left_interface.index_bias = 0;
        right_interface.index_bias = left_interface.nodes_count();
        interface.children = Some((
            (left_weak, left_interface.nodes_count()),
            (right_weak, right_interface.nodes_count()),
        ));
        for other_interface in [left_interface, right_interface] {
            interface.sum_dual_variables += other_interface.sum_dual_variables;
            interface.sum_grow_speed += other_interface.sum_grow_speed;
        }
    }

    /// do a sanity check of if all the nodes are in consistent state
    #[inline(never)]
    #[allow(clippy::unnecessary_cast)]
    #[allow(clippy::needless_borrow)]
    pub fn sanity_check(&self) -> Result<Vec<Option<DualNodePtr>>, String> {
        let mut flattened_nodes = vec![];
        self.flatten_nodes(&mut flattened_nodes);
        let interface = self.read_recursive();
        if false {
            eprintln!("[warning] sanity check disabled for dual_module.rs");
            return Ok(flattened_nodes);
        }
        let mut visited_syndrome = HashSet::with_capacity((interface.nodes_count() * 2) as usize);
        let mut sum_individual_dual_variable = 0;
        for (index, dual_node_ptr) in flattened_nodes.iter().enumerate() {
            if let Some(dual_node_ptr) = dual_node_ptr {
                let dual_node = dual_node_ptr.read_recursive();
                sum_individual_dual_variable += dual_node.get_dual_variable(&interface);
                if dual_node.index != index as NodeIndex {
                    return Err(format!(
                        "dual node index wrong: expected {}, actual {}",
                        index, dual_node.index
                    ));
                }
                match &dual_node.class {
                    DualNodeClass::Blossom {
                        nodes_circle,
                        touching_children,
                    } => {
                        for (idx, circle_node_weak) in nodes_circle.iter().enumerate() {
                            let circle_node_ptr = circle_node_weak.upgrade_force();
                            if &circle_node_ptr == dual_node_ptr {
                                return Err("a blossom should not contain itself".to_string());
                            }
                            let circle_node = circle_node_ptr.read_recursive();
                            if circle_node.parent_blossom.as_ref() != Some(&dual_node_ptr.downgrade()) {
                                return Err(format!(
                                    "blossom {} contains {} but child's parent pointer = {:?} is not pointing back",
                                    dual_node.index, circle_node.index, circle_node.parent_blossom
                                ));
                            }
                            if circle_node.grow_state != DualNodeGrowState::Stay {
                                return Err(format!("child node {} is not at Stay state", circle_node.index));
                            }
                            // check if circle node is still tracked, i.e. inside self.nodes
                            if circle_node.index >= interface.nodes_count()
                                || interface.get_node(circle_node.index).is_none()
                            {
                                return Err(format!("child's index {} is not in the interface", circle_node.index));
                            }
                            let tracked_circle_node_ptr = interface.get_node(circle_node.index).unwrap();
                            if tracked_circle_node_ptr != circle_node_ptr {
                                return Err(format!(
                                    "the tracked ptr of child {} is not what's being pointed",
                                    circle_node.index
                                ));
                            }
                            // check children belongings
                            let (child_weak_1, child_weak_2) = &touching_children[idx];
                            if matches!(circle_node.class, DualNodeClass::DefectVertex { .. }) {
                                if child_weak_1 != circle_node_weak {
                                    return Err(format!("touching child can only be syndrome node {}", circle_node.index));
                                }
                                if child_weak_2 != circle_node_weak {
                                    return Err(format!("touching child can only be syndrome node {}", circle_node.index));
                                }
                            } else {
                                let child_ptr_1 = child_weak_1.upgrade_force();
                                let child_ptr_2 = child_weak_2.upgrade_force();
                                let child_1_ancestor = child_ptr_1.get_ancestor_blossom();
                                let child_2_ancestor = child_ptr_2.get_ancestor_blossom();
                                let circle_ancestor = circle_node_ptr.get_ancestor_blossom();
                                if child_1_ancestor != circle_ancestor {
                                    return Err(format!("{:?} is not descendent of {}", child_ptr_1, circle_node.index));
                                }
                                if child_2_ancestor != circle_ancestor {
                                    return Err(format!("{:?} is not descendent of {}", child_ptr_2, circle_node.index));
                                }
                            }
                        }
                    }
                    DualNodeClass::DefectVertex { defect_index } => {
                        if visited_syndrome.contains(defect_index) {
                            return Err(format!("duplicate defect index: {}", defect_index));
                        }
                        visited_syndrome.insert(*defect_index);
                    }
                }
                if let Some(parent_blossom_weak) = &dual_node.parent_blossom {
                    if dual_node.grow_state != DualNodeGrowState::Stay {
                        return Err(format!("child node {} is not at Stay state", dual_node.index));
                    }
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
                                return Err(format!(
                                    "{} is the parent of {} but the child only presents {} times",
                                    parent_blossom.index, dual_node.index, found_match_count
                                ));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "{}, as the parent of {}, is not a blossom",
                                parent_blossom.index, dual_node.index
                            ))
                        }
                    }
                    // check if blossom is still tracked, i.e. inside interface.nodes
                    if parent_blossom.index >= interface.nodes_count() || interface.get_node(parent_blossom.index).is_none()
                    {
                        return Err(format!(
                            "parent blossom's index {} is not in the interface",
                            parent_blossom.index
                        ));
                    }
                    let tracked_parent_blossom_ptr = interface.get_node(parent_blossom.index).unwrap();
                    if tracked_parent_blossom_ptr != parent_blossom_ptr {
                        return Err(format!(
                            "the tracked ptr of parent blossom {} is not what's being pointed",
                            parent_blossom.index
                        ));
                    }
                }
            }
        }
        if sum_individual_dual_variable != interface.sum_dual_variables {
            return Err(format!(
                "internal error: the sum of dual variables is {} but individual sum is {}",
                interface.sum_dual_variables, sum_individual_dual_variable
            ));
        }
        Ok(flattened_nodes)
    }

    pub fn sum_dual_variables(&self) -> Weight {
        self.read_recursive().sum_dual_variables
    }
}

impl Ord for MaxUpdateLength {
    fn cmp(&self, other: &Self) -> Ordering {
        debug_assert!(
            !matches!(self, MaxUpdateLength::NonZeroGrow(_)),
            "priority ordering is not valid for NonZeroGrow"
        );
        debug_assert!(
            !matches!(other, MaxUpdateLength::NonZeroGrow(_)),
            "priority ordering is not valid for NonZeroGrow"
        );
        if self == other {
            return Ordering::Equal;
        }
        // VertexShrinkStop has the lowest priority: it should be put at the end of any ordered list
        // this is because solving VertexShrinkStop conflict is not possible, but when this happens, the primal module
        // should have put this node as a "-" node in the alternating tree, so there must be a parent and a child that
        // are "+" nodes, conflicting with each other at exactly this VertexShrinkStop node. In this case, as long as
        // one solves those "+" nodes conflicting, e.g. forming a blossom, this node's VertexShrinkStop conflict is automatically solved
        match (
            matches!(self, MaxUpdateLength::VertexShrinkStop(..)),
            matches!(other, MaxUpdateLength::VertexShrinkStop(..)),
        ) {
            (true, false) => return Ordering::Less,    // less priority
            (false, true) => return Ordering::Greater, // greater priority
            (true, true) => {
                return self
                    .get_vertex_shrink_stop()
                    .unwrap()
                    .cmp(&other.get_vertex_shrink_stop().unwrap())
            } // don't care, just compare pointer
            _ => {}
        }
        // then, blossom expanding has the low priority, because it's infrequent and expensive
        match (
            matches!(self, MaxUpdateLength::BlossomNeedExpand(..)),
            matches!(other, MaxUpdateLength::BlossomNeedExpand(..)),
        ) {
            (true, false) => return Ordering::Less,    // less priority
            (false, true) => return Ordering::Greater, // greater priority
            (true, true) => {
                return self
                    .get_blossom_need_expand()
                    .unwrap()
                    .cmp(&other.get_blossom_need_expand().unwrap())
            } // don't care, just compare pointer
            _ => {}
        }
        // We'll prefer match nodes internally instead of to boundary, because there might be less path connecting to boundary
        // this is only an attempt to optimize the MWPM decoder, but anyway it won't be an optimal decoder
        match (
            matches!(self, MaxUpdateLength::TouchingVirtual(..)),
            matches!(other, MaxUpdateLength::TouchingVirtual(..)),
        ) {
            (true, false) => return Ordering::Less,    // less priority
            (false, true) => return Ordering::Greater, // greater priority
            (true, true) => {
                let (a, c) = self.get_touching_virtual().unwrap();
                let (b, d) = other.get_touching_virtual().unwrap();
                return a.cmp(&b).reverse().then(c.cmp(&d).reverse());
            } // don't care, just compare pointer
            _ => {}
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
    /// update all the interface nodes to be up-to-date
    pub fn update(&self) {
        match self {
            Self::NonZeroGrow(_) => {}
            Self::Conflicting((a, b), (c, d)) => {
                for x in [a, b, c, d] {
                    x.update();
                }
            }
            Self::TouchingVirtual((a, b), _) => {
                for x in [a, b] {
                    x.update();
                }
            }
            Self::BlossomNeedExpand(x) => {
                x.update();
            }
            Self::VertexShrinkStop((a, option)) => {
                a.update();
                if let Some((b, c)) = option {
                    b.update();
                    c.update();
                }
            }
        }
    }

    /// useful function to assert expected case
    #[allow(dead_code)]
    pub fn is_conflicting(&self, a: &DualNodePtr, b: &DualNodePtr) -> bool {
        if let MaxUpdateLength::Conflicting((n1, _), (n2, _)) = self {
            if n1 == a && n2 == b {
                return true;
            }
            if n1 == b && n2 == a {
                return true;
            }
        }
        false
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_none_zero_growth(&self) -> Option<Weight> {
        match self {
            Self::NonZeroGrow((length, _has_empty_boundary_node)) => Some(*length),
            _ => None,
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_conflicting(&self) -> Option<(DualNodePtr, DualNodePtr)> {
        match self {
            Self::Conflicting((a, _), (b, _)) => Some((a.clone(), b.clone())),
            _ => None,
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_touching_virtual(&self) -> Option<(DualNodePtr, VertexIndex)> {
        match self {
            Self::TouchingVirtual((a, _), (b, _)) => Some((a.clone(), *b)),
            _ => None,
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_blossom_need_expand(&self) -> Option<DualNodePtr> {
        match self {
            Self::BlossomNeedExpand(a) => Some(a.clone()),
            _ => None,
        }
    }

    /// helper function that get values out of the enum
    #[allow(dead_code)]
    #[inline(always)]
    pub fn get_vertex_shrink_stop(&self) -> Option<DualNodePtr> {
        match self {
            Self::VertexShrinkStop((a, _)) => Some(a.clone()),
            _ => None,
        }
    }
}

/// temporarily remember the weights that has been changed, so that it can revert back
#[derive(Debug, Clone)]
pub struct EdgeWeightModifier {
    /// edge with changed weighted caused by the erasure or X/Z correlation
    pub modified: Vec<(EdgeIndex, Weight)>,
}

impl Default for EdgeWeightModifier {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeWeightModifier {
    pub fn new() -> Self {
        Self { modified: vec![] }
    }

    /// record the modified edge
    pub fn push_modified_edge(&mut self, erasure_edge: EdgeIndex, original_weight: Weight) {
        self.modified.push((erasure_edge, original_weight));
    }

    /// if some edges are not recovered
    pub fn has_modified_edges(&self) -> bool {
        !self.modified.is_empty()
    }

    /// retrieve the last modified edge, panic if no more modified edges
    pub fn pop_modified_edge(&mut self) -> (EdgeIndex, Weight) {
        self.modified
            .pop()
            .expect("no more modified edges, please check `has_modified_edges` before calling this method")
    }
}

impl std::ops::Deref for EdgeWeightModifier {
    type Target = Vec<(EdgeIndex, Weight)>;

    fn deref(&self) -> &Self::Target {
        &self.modified
    }
}
