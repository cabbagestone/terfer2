mod edge;
mod instance;
pub mod node;

#[cfg(test)]
mod test {
    use super::node::{NodeApi, NodeRef};

    #[test]
    fn test_node() {
        let node = NodeRef::new_ref("Parent 1".to_string());
        assert_eq!(node.value().unwrap(), "Parent 1");

        let child = NodeRef::new_ref("Child 1".to_string());
        assert_eq!(child.value().unwrap(), "Child 1");

        NodeRef::connect_parent_child(node.clone(), child.clone()).unwrap();
        assert!(node.is_parent_of(child));
    }
}
