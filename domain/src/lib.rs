use std::fmt::Display;
use std::sync::{Arc, RwLock, Weak};
use jiff::Zoned;
use uuid::Uuid;

struct Node {
    id: String,
    created_at: Zoned,
    deleted_at: Option<Zoned>,
    instances: Vec<Instance>,
    edges: Vec<EdgeRef>,
}

type NodeRef = Arc<RwLock<Node>>;
type WeakNodeRef = Weak<RwLock<Node>>;
type EdgeRef = Arc<RwLock<Edge>>;

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

impl From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Node>>> for NodeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Node>>) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>> for NodeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Edge>>> for NodeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Edge>>) -> NodeError {
        NodeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Edge>>> for NodeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Edge>>) -> NodeError {
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
            Ok(instance) => Ok(&instance.value),
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
            Some(edge) => Ok(edge.write()?.delete()?),
            None => Err(NodeError::EdgeNotFound)
        }
    }
    
    pub fn edge_count(&self) -> Result<usize, NodeError> {
        Ok(self.edges().count())
    }
}

struct Edge {
    id: String,
    parent: WeakNodeRef, 
    child: NodeRef,
    created_at: Zoned,
    deleted_at: Option<Zoned>
}

#[derive(Debug, PartialEq, Eq)]
enum EdgeError {
    DeleteDeletedEdge,
    RestoreNotDeletedEdge,
    RwLockError(String),
    WeakReferenceUpgradeFailed
}

impl std::error::Error for EdgeError {}

impl Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EdgeError::DeleteDeletedEdge => write!(f, "Cannot delete an already deleted edge"),
            EdgeError::RestoreNotDeletedEdge => write!(f, "Cannot restore an edge that is not deleted"),
            EdgeError::RwLockError(message) => write!(f, "Read/Write Lock error: {}", message),
            EdgeError::WeakReferenceUpgradeFailed => write!(f, "Failed to upgrade weak reference")
        }
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>> for EdgeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Node>>> for EdgeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Node>>) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Edge>>> for EdgeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Edge>>) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Edge>>> for EdgeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, Edge>>) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

trait EdgeApi {
    fn read_child(&self) -> NodeRef;
    fn read_parent(&self) -> NodeRef;
    fn is_live(&self) -> bool;
    fn delete(&mut self) -> Result<(), EdgeError>;
    fn restore(&mut self) -> Result<(), EdgeError>;
}

impl EdgeApi for Arc<RwLock<Edge>> {
    fn read_child(&self) -> NodeRef {
        self.read().unwrap().child.clone()
    }
    
    fn read_parent(&self) -> NodeRef {
        self.read().unwrap().parent.upgrade().unwrap().clone()
    }
    
    fn is_live(&self) -> bool {
        self.read().unwrap().is_live()
    }
    
    fn delete(&mut self) -> Result<(), EdgeError> {
        self.write()?.delete()
    }
    
    fn restore(&mut self) -> Result<(), EdgeError> {
        self.write()?.restore()
    }
}

impl Edge {
    fn new_ref(from: NodeRef, to: NodeRef) -> EdgeRef {
        Arc::new(RwLock::new(Edge::new(Arc::downgrade(&from.clone()), to.clone())))
    }
    
    fn new(parent: WeakNodeRef, child: NodeRef) -> Edge {
        Edge {
            id: Uuid::new_v4().to_string(),
            parent,
            child,
            created_at: Zoned::now(),
            deleted_at: None
        }
    }

    fn panic_on_poison_eq(edge1: &EdgeRef, edge2: &EdgeRef) -> bool {
        edge1.read().unwrap().id == edge2.read().unwrap().id
    }
    
    fn is_live(&self) -> bool {
        self.deleted_at.is_none() &&
        !self.child.read().unwrap().is_deleted() &&
        self.parent.upgrade().is_some_and(|p| !p.read().unwrap().is_deleted())
    }
    
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    
    fn delete(&mut self) -> Result<(), EdgeError> {
        if self.is_deleted() {
            return Err(EdgeError::DeleteDeletedEdge);
        }
        
        self.deleted_at = Some(Zoned::now());
        Ok(())
    }
    
    fn restore(&mut self) -> Result<(), EdgeError> {
        if !self.is_deleted() {
            return Err(EdgeError::RestoreNotDeletedEdge);
        }
        
        self.deleted_at = None;
        Ok(())
    }
}

struct Instance {
    saved_at: Zoned,
    instance_type: InstanceType,
    value: String
}

impl Instance {
    fn new_created(value: String) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Created,
            value
        }
    }
    
    fn new_updated(value: String) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Updated,
            value
        }
    }
    
    fn deleted_child(&self) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Deleted,
            value: self.value.clone()
        }
    }
    
    fn restored_child(&self) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Restored,
            value: self.value.clone()
        }
    }
}

enum InstanceType {
    Created,
    Deleted,
    Restored,
    Updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_node() {
        let node = Node::new("value".to_string());

        assert_eq!(node.instances.len(), 1);
        assert_eq!(node.edges.len(), 0);
        assert_eq!(node.is_deleted(), false);
    }

    #[test]
    fn test_update_node() {
        let mut node = Node::new("value".to_string());

        node.update("new value".to_string()).unwrap();

        assert_eq!(node.instances.len(), 2);
        assert_eq!(node.is_deleted(), false);
    }

    #[test]
    fn test_delete_node() {
        let mut node = Node::new("value".to_string());

        node.delete().unwrap();

        assert_eq!(node.instances.len(), 2);
        assert_eq!(node.is_deleted(), true);
    }

    #[test]
    fn test_restore_node() {
        let mut node = Node::new("value".to_string());

        node.delete().unwrap();
        node.restore().unwrap();

        assert_eq!(node.instances.len(), 3);
        assert_eq!(node.is_deleted(), false);
    }

    #[test]
    fn test_value() {
        let node = Node::new("value".to_string());

        assert_eq!(node.value().unwrap(), "value");
    }

    #[test]
    fn test_connect_to() {
        let mut node1 = Node::new("value1".to_string());
        let node2 = Node::new("value2".to_string());

        node1.make_parent_of(&node2).unwrap();

        assert_eq!(node1.edges.len(), 1);
        assert_eq!(node1.is_parent_of(&node2), true);
    }

    #[test]
    fn test_disconnect_from() {
        let mut node1 = Node::new("value1".to_string());
        let node2 = Node::new("value2".to_string());

        node1.make_parent_of(&node2);
        node1.remove_child(&node2).unwrap();

        assert_eq!(node1.edge_count(), 0);
        assert_eq!(node1.is_parent_of(&node2), false);
    }
    
    #[test]
    fn test_everything() {
        let mut node1 = Node::new("value1".to_string());
        let mut node2 = Node::new("value2".to_string());
        
        node1.make_parent_of(&node2).unwrap();
        node1.update("new value1".to_string()).unwrap();
        node2.update("new value2".to_string()).unwrap();
        node1.delete().unwrap();
        node2.delete().unwrap();
        node1.restore().unwrap();
        node2.restore().unwrap();
        node1.remove_child(&node2).unwrap();
        
        assert_eq!(node1.instances.len(), 6);
        assert_eq!(node1.edges.len(), 0);
        assert_eq!(node1.is_deleted(), false);
        assert_eq!(node1.value().unwrap(), "new value1");
        assert_eq!(node1.edge_count(), 0);
    }
}