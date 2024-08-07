mod edge;
mod instance;
pub mod node;

#[cfg(test)]
mod test {
    use super::node::{NodeApi, NodeRef};

    #[test]
    fn test_node() {
        let parent = NodeRef::new_ref("Parent 1".to_string());
        assert_eq!(parent.value().unwrap(), "Parent 1");

        let child = NodeRef::new_ref("Child 1".to_string());

        NodeRef::connect_parent_child(parent.clone(), child.clone()).unwrap();
        assert!(parent.is_parent_of(child.clone()));

        let parent2 = NodeRef::new_ref("Parent 2".to_string());
        let p2 = parent2.clone();
        let c = child.clone();
        let connection_result = NodeRef::connect_parent_child(p2, c);
        connection_result.unwrap();
    }
}
