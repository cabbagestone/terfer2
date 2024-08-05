use std::fmt::Display;
use std::sync::{Arc, RwLock};
use jiff::Zoned;
use uuid::Uuid;

struct Node {
    id: String,
    created_at: Zoned,
    deleted_at: Option<Zoned>,
    instances: Vec<Instance>,
    edges: Vec<Edge>
}

type NodeRef = Arc<RwLock<Node>>;

#[derive(Debug, PartialEq, Eq)]
enum NodeError {
    OperationOnEmptyNode,
    DeleteDeletedNode,
    OperationOnDeletedNode,
    RestoreNotDeletedNode,
    EdgeNotFound,
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
            NodeError::Edge(error) => write!(f, "Edge error: {}", error)
        }
    }
}

impl From<EdgeError> for NodeError {
    fn from(error: EdgeError) -> NodeError {
        NodeError::Edge(error)
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
    
    fn ok_refs_eq(node1: &NodeRef, node2: &NodeRef) -> bool {
        node1.read().is_ok_and(|n1| node2.read().is_ok_and(|n2| n1.id == n2.id))
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
    
    pub fn edges_mut(&mut self) -> impl Iterator<Item = &mut Edge> {
        // Poisoned lock is just discarded and not counted
        self.edges.iter_mut().filter(|edge| edge.is_live().unwrap_or(false))
    }
    
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        // Poisoned lock is just discarded and not counted
        self.edges.iter().filter(|edge| edge.is_live().unwrap_or(false))
    }
    
    pub fn dead_edges_mut(&mut self) -> impl Iterator<Item = &mut Edge> {
        // Poisoned lock is just discarded and not counted
        self.edges.iter_mut().filter(|edge| !edge.is_live().unwrap_or(false))
    }
    
    pub fn connect_to(&mut self, node: NodeRef) -> Result<(), NodeError> {
        self.deleted_check()?;
        
        if Ok(()) == self.restore_connection(node.clone()) {
            return Ok(());
        }
        
        self.edges.push(Edge::new(node.clone()));
        
        Ok(())
    }
    
    pub fn is_connected_to(&self, connection: NodeRef) -> bool {
        self.edges().any(|edge| Node::ok_refs_eq(&edge.to, &connection))
    }
    
    pub fn disconnect_from(&mut self, node: NodeRef) -> Result<(), NodeError> {
        self.deleted_check()?;
        
        match self.edges_mut().find(|edge| Node::ok_refs_eq(&edge.to, &node)) {
            Some(edge) => Ok(edge.delete()?),
            None => Err(NodeError::EdgeNotFound)
        }
    }
    
    pub fn restore_connection(&mut self, node: NodeRef) -> Result<(), NodeError> {
        self.deleted_check()?;
        
        match self.dead_edges_mut().find(|edge| Node::ok_refs_eq(&edge.to, &node)) {
            Some(edge) => Ok(edge.restore()?),
            None => Err(NodeError::EdgeNotFound)
        }
    }
    
    pub fn connection_count(&self) -> Result<usize, NodeError> {
        Ok(self.edges().count())
    }
}

struct Edge {
    to: NodeRef,
    created_at: Zoned,
    deleted_at: Option<Zoned>
}

#[derive(Debug, PartialEq, Eq)]
enum EdgeError {
    DeleteDeletedEdge,
    RestoreNotDeletedEdge,
    RwLockError(String)
}

impl std::error::Error for EdgeError {}

impl Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EdgeError::DeleteDeletedEdge => write!(f, "Cannot delete an already deleted edge"),
            EdgeError::RestoreNotDeletedEdge => write!(f, "Cannot restore an edge that is not deleted"),
            EdgeError::RwLockError(message) => write!(f, "Read/Write Lock error: {}", message)
        }
    }
}

impl From<std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>> for EdgeError {
    fn from(error: std::sync::PoisonError<std::sync::RwLockReadGuard<'_, Node>>) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl Edge {
    fn new(to: NodeRef) -> Edge {
        Edge {
            to,
            created_at: Zoned::now(),
            deleted_at: None
        }
    }
    
    fn is_live(&self) -> Result<bool, EdgeError> {
        Ok(self.deleted_at.is_none() && !self.to.read()?.is_deleted())
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

        node1.connect_to(&node2).unwrap();

        assert_eq!(node1.edges.len(), 1);
        assert_eq!(node1.is_connected_to(&node2), true);
    }

    #[test]
    fn test_disconnect_from() {
        let mut node1 = Node::new("value1".to_string());
        let node2 = Node::new("value2".to_string());

        node1.connect_to(&node2);
        node1.disconnect_from(&node2).unwrap();

        assert_eq!(node1.connection_count(), 0);
        assert_eq!(node1.is_connected_to(&node2), false);
    }
    
    #[test]
    fn test_everything() {
        let mut node1 = Node::new("value1".to_string());
        let mut node2 = Node::new("value2".to_string());
        
        node1.connect_to(&node2).unwrap();
        node1.update("new value1".to_string()).unwrap();
        node2.update("new value2".to_string()).unwrap();
        node1.delete().unwrap();
        node2.delete().unwrap();
        node1.restore().unwrap();
        node2.restore().unwrap();
        node1.disconnect_from(&node2).unwrap();
        
        assert_eq!(node1.instances.len(), 6);
        assert_eq!(node1.edges.len(), 0);
        assert_eq!(node1.is_deleted(), false);
        assert_eq!(node1.value().unwrap(), "new value1");
        assert_eq!(node1.connection_count(), 0);
    }
}