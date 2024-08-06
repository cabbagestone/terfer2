use std::fmt::Display;
use std::sync::{Arc, RwLock, Weak};
use jiff::Zoned;
use uuid::Uuid;
use super::edge::{Edge, EdgeApi, EdgeError, EdgeReadPoisonError, EdgeRef, EdgeWritePoisonError};
use super::instance::Instance;

pub(crate) struct Node {
    id: String,
    created_at: Zoned,
    deleted_at: Option<Zoned>,
    instances: Vec<Instance>,
    edges: Vec<EdgeRef>,
}

pub(crate) type NodeRef = Arc<RwLock<Node>>;
pub(crate) type WeakNodeRef = Weak<RwLock<Node>>;
pub(crate) type NodeReadPoisonError<'a> = std::sync::PoisonError<std::sync::RwLockReadGuard<'a, Node>>;
pub(crate) type NodeWritePoisonError<'a> = std::sync::PoisonError<std::sync::RwLockWriteGuard<'a, Node>>;

#[derive(Debug, PartialEq, Eq)]
enum NodeError {
    OperationOnEmptyNode,
    DeleteDeletedNode,
    OperationOnDeletedNode,
    RestoreNotDeletedNode,
    EdgeNotFound,
    RwLockError(String),
    Edge(EdgeError)
}

impl std::error::Error for NodeError {}

impl Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NodeError::OperationOnEmptyNode => write!(f, "Cannot perform an operation on an empty node"),
            NodeError::OperationOnDeletedNode => write!(f, "Cannot perform an operation on a deleted node"),
            NodeError::DeleteDeletedNode => write!(f, "Cannot delete an already deleted node"),
            NodeError::RestoreNotDeletedNode => write!(f, "Cannot restore a node that is not deleted"),
            NodeError::EdgeNotFound => write!(f, "No related node found"),
            NodeError::Edge(error) => write!(f, "Edge error: {}", error),
            NodeError::RwLockError(message) => write!(f, "Read/Write lock error: {}", message)
        }
    }
}

impl From<EdgeError> for NodeError {
    fn from(error: EdgeError) -> NodeError {
        NodeError::Edge(error)
    }
}

impl From<NodeWritePoisonError<'_>> for NodeError {
    fn from(error: NodeWritePoisonError) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<NodeReadPoisonError<'_>> for NodeError {
    fn from(error: NodeReadPoisonError) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<EdgeWritePoisonError<'_>> for NodeError {
    fn from(error: EdgeWritePoisonError) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<EdgeReadPoisonError<'_>> for NodeError {
    fn from(error: EdgeReadPoisonError) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl Node {
    pub fn new(value: String) -> Node {
        Node {
            id: Uuid::new_v4().to_string(),
            created_at: Zoned::now(),
            deleted_at: None,
            instances: Vec::from([Instance::new_created(value)]),
            edges: Vec::new()
        }
    }

    fn panic_on_poison_eq(node1: NodeRef, node2: NodeRef) -> bool {
        node1.read().unwrap().id == node2.read().unwrap().id
    }

    fn last_instance(&self) -> Result<&Instance, NodeError> {
        match self.instances.last() {
            Some(instance) => Ok(instance),
            None => Err(NodeError::OperationOnEmptyNode)
        }
    }

    pub fn update(&mut self, value: String) -> Result<(), NodeError> {
        self.deleted_check()?;

        self.instances.push(Instance::new_updated(value));
        Ok(())
    }

    pub fn delete(&mut self) -> Result<(), NodeError> {
        if self.is_deleted() {
            return Err(NodeError::DeleteDeletedNode);
        }

        self.deleted_at = Some(Zoned::now());

        match self.last_instance() {
            Ok(instance) => {
                self.instances.push(instance.deleted_child());
                Ok(())
            },
            Err(_) => Err(NodeError::OperationOnEmptyNode)
        }
    }

    pub fn restore(&mut self) -> Result<(), NodeError> {
        if !self.is_deleted() {
            return Err(NodeError::RestoreNotDeletedNode);
        }

        self.deleted_at = None;

        match self.last_instance() {
            Ok(instance) => {
                self.instances.push(instance.restored_child());
                Ok(())
            },
            Err(_) => Err(NodeError::OperationOnEmptyNode)
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn deleted_check(&self) -> Result<(), NodeError> {
        if self.is_deleted() {
            return Err(NodeError::OperationOnDeletedNode);
        }

        Ok(())
    }

    pub fn value(&self) -> Result<&str, NodeError> {
        match self.last_instance() {
            Ok(instance) => Ok(&instance.value()),
            Err(_) => Err(NodeError::OperationOnEmptyNode)
        }
    }

    pub fn edges_mut(&mut self) -> impl Iterator<Item = &mut EdgeRef> {
        self.edges.iter_mut().filter(|edge| edge.is_live())
    }

    pub fn edges(&self) -> impl Iterator<Item = &EdgeRef> {
        self.edges.iter().filter(|edge| edge.is_live())
    }

    pub fn dead_edges_mut(&mut self) -> impl Iterator<Item = &mut EdgeRef> {
        self.edges.iter_mut().filter(|edge| !edge.is_live())
    }

    pub fn make_parent_of(ref_to_parent: NodeRef, ref_to_child: NodeRef) -> Result<(), NodeError> {
        let mut parent = ref_to_parent.write()?;
        parent.deleted_check()?;

        let mut child = ref_to_child.write()?;
        child.deleted_check()?;

        let edge = Edge::new_ref(ref_to_parent.clone(), ref_to_child.clone());
        parent.add_or_restore_edge(edge.clone())?;
        child.add_or_restore_edge(edge.clone())?;

        Ok(())
    }

    fn add_or_restore_edge(&mut self, edge: EdgeRef) -> Result<(), NodeError> {
        let value: Option<&mut EdgeRef>;
        {
            value = self.dead_edges_mut().find(|e| Edge::panic_on_poison_eq(e, &edge));
        }

        match value {
            Some(edge) => Ok(edge.restore()?),
            None => {
                self.edges.push(edge);
                Ok(())
            }
        }
    }

    pub fn is_parent_of(&self, connection: NodeRef) -> bool {
        self.edges().any(
            |edge| Node::panic_on_poison_eq(edge.read_child(), connection.clone()))
    }

    pub fn remove_child(&mut self, node: NodeRef) -> Result<(), NodeError> {
        self.deleted_check()?;

        match self.edges_mut().find(|edge| Node::panic_on_poison_eq(edge.read_child(), node.clone())) {
            Some(edge) => Ok(edge.delete()?),
            None => Err(NodeError::EdgeNotFound)
        }
    }

    pub fn edge_count(&self) -> Result<usize, NodeError> {
        Ok(self.edges().count())
    }
}