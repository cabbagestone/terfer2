use petgraph::{graph::NodeIndex, Graph};

pub struct Node {
    id: String,
    name: String,
}

pub struct Edge {
    id: String,
}

pub type TerferGraph = Graph<Node, Edge>;

pub trait Terfer {
    fn new_tg() -> Self;
    fn add_node(&mut self, node: Node) -> NodeIndex;
    fn add_edge(&mut self, a: NodeIndex, b: NodeIndex, edge: Edge);
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
}

impl Terfer for TerferGraph {
    fn new_tg() -> Self {
        TerferGraph::new()
    }

    fn add_node(&mut self, node: Node) -> NodeIndex {
        self.add_node(node)
    }

    fn add_edge(&mut self, a: NodeIndex, b: NodeIndex, edge: Edge) {
        self.add_edge(a, b, edge);
    }

    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn edge_count(&self) -> usize {
        self.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut tg = TerferGraph::new_tg();
        let node1 = Node {
            id: "1".to_string(),
            name: "Node 1".to_string(),
        };
        let node2 = Node {
            id: "2".to_string(),
            name: "Node 2".to_string(),
        };
        let node_index1 = tg.add_node(node1);
        let node_index2 = tg.add_node(node2);
        let edge = Edge {
            id: "1".to_string(),
        };
        tg.add_edge(node_index1, node_index2, edge);
        assert_eq!(tg.node_count(), 2);
        assert_eq!(tg.edge_count(), 1);
    }
}
