use std::hint::black_box;

use example_crate::{Graph, TraceResult};

#[harness::bench]
#[derive(Default)]
struct Trace {
    graph: Graph,
    result: TraceResult,
}

impl harness::Benchmark for Trace {
    fn prologue(&mut self) {
        self.graph = black_box(Graph::generate::<42>(1000, 1000000, 1100000));
    }

    fn iter(&mut self) {
        self.result = self.graph.trace();
    }

    fn epilogue(&mut self) {
        println!(
            "Marked {} nodes; traced {} edges",
            self.result.num_marked_nodes, self.result.num_traced_edges
        );
        assert_eq!(self.result.num_marked_nodes, 181234);
        assert_eq!(self.result.num_traced_edges, 200135);
    }
}
