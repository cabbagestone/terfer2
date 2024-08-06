use std::sync::{Arc, RwLock};
use jiff::Zoned;
use uuid::Uuid;
use super::node::{NodeRef, WeakNodeRef, NodeWritePoisonError, NodeReadPoisonError};

pub(crate) type EdgeRef = Arc<RwLock<Edge>>;
pub(crate) type EdgeWritePoisonError<'a> = std::sync::PoisonError<std::sync::RwLockWriteGuard<'a, Edge>>;
pub(crate) type EdgeReadPoisonError<'a> = std::sync::PoisonError<std::sync::RwLockReadGuard<'a, Edge>>;

pub(crate) struct Edge {
    id: String,
    parent: WeakNodeRef,
    child: NodeRef,
    created_at: Zoned,
    deleted_at: Option<Zoned>
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EdgeError {
    DeleteDeletedEdge,
    RestoreNotDeletedEdge,
    RwLockError(String),
    WeakReferenceUpgradeFailed
}

impl std::error::Error for EdgeError {}

impl std::fmt::Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EdgeError::DeleteDeletedEdge => write!(f, "Cannot delete an already deleted edge"),
            EdgeError::RestoreNotDeletedEdge => write!(f, "Cannot restore an edge that is not deleted"),
            EdgeError::RwLockError(message) => write!(f, "Read/Write Lock error: {}", message),
            EdgeError::WeakReferenceUpgradeFailed => write!(f, "Failed to upgrade weak reference")
        }
    }
}

impl From<NodeWritePoisonError<'_>> for EdgeError {
    fn from(error: NodeWritePoisonError) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<NodeReadPoisonError<'_>> for EdgeError {
    fn from(error: NodeReadPoisonError) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<EdgeReadPoisonError<'_>> for EdgeError {
    fn from(error: EdgeReadPoisonError) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

impl From<EdgeWritePoisonError<'_>> for EdgeError {
    fn from(error: EdgeWritePoisonError) -> EdgeError {
        EdgeError::RwLockError(error.to_string())
    }
}

pub trait EdgeApi {
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
    pub(crate) fn new_ref(from: NodeRef, to: NodeRef) -> EdgeRef {
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

    pub(crate) fn panic_on_poison_eq(edge1: &EdgeRef, edge2: &EdgeRef) -> bool {
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