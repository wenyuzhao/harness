use std::{collections::VecDeque, sync::atomic::AtomicBool};

use rand::{rngs::StdRng, Rng, SeedableRng};

#[derive(Default)]
pub struct Graph {
    pub roots: Vec<usize>,
    pub nodes: Vec<Node>,
}

pub struct Node {
    pub mark: AtomicBool,
    pub edges: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct TraceResult {
    pub num_marked_nodes: usize,
    pub num_traced_edges: usize,
}

impl Graph {
    pub fn generate<const SEED: u64>(num_roots: usize, num_nodes: usize, num_edges: usize) -> Self {
        let mut rng = StdRng::seed_from_u64(SEED);
        // create nodes
        let mut nodes = Vec::with_capacity(num_nodes);
        for _ in 0..num_nodes {
            nodes.push(Node {
                mark: AtomicBool::new(false),
                edges: Vec::new(),
            });
        }
        // randomly select distinct root nodes
        let mut roots = Vec::with_capacity(num_roots);
        for _ in 0..num_roots {
            let root = rng.gen_range(0..num_nodes);
            roots.push(root);
        }
        // randomly connect nodes
        for _ in 0..num_edges {
            let a = rng.gen_range(0..num_nodes);
            let b = rng.gen_range(0..num_nodes);
            nodes[a].edges.push(b);
        }
        Self { roots, nodes }
    }

    fn trace_node_enqueuing(&self) -> TraceResult {
        let mut counters = TraceResult::default();
        let mut buffer = VecDeque::new();
        for root in &self.roots {
            counters.num_traced_edges += 1;
            let node = &self.nodes[*root];
            if !node.mark.swap(true, std::sync::atomic::Ordering::SeqCst) {
                counters.num_marked_nodes += 1;
                buffer.push_back(*root);
            }
        }
        while let Some(node) = buffer.pop_front() {
            let node = &self.nodes[node];
            for edge in &node.edges {
                counters.num_traced_edges += 1;
                let child = &self.nodes[*edge];
                if !child.mark.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    counters.num_marked_nodes += 1;
                    buffer.push_back(*edge);
                }
            }
        }
        counters
    }

    fn trace_edge_enqueuing(&self) -> TraceResult {
        let mut counters = TraceResult::default();
        let mut buffer = VecDeque::new();
        for root in &self.roots {
            counters.num_traced_edges += 1;
            buffer.push_back(*root);
        }
        while let Some(node) = buffer.pop_front() {
            let node = &self.nodes[node];
            if node.mark.swap(true, std::sync::atomic::Ordering::SeqCst) {
                continue;
            }
            counters.num_marked_nodes += 1;
            for edge in &node.edges {
                counters.num_traced_edges += 1;
                buffer.push_back(*edge);
            }
        }
        counters
    }

    pub fn trace(&self) -> TraceResult {
        if cfg!(feature = "node_enqueuing") {
            self.trace_node_enqueuing()
        } else if cfg!(feature = "edge_enqueuing") {
            self.trace_edge_enqueuing()
        } else {
            unimplemented!()
        }
    }
}
