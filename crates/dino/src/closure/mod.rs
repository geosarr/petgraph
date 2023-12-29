mod union;
mod unique_vec;

use croaring::Bitmap as RoaringBitmap;
use either::Either;
use fnv::FnvBuildHasher;
use hashbrown::HashMap;

pub(crate) use self::{union::UnionIterator, unique_vec::UniqueVec};
use crate::{
    edge::{Edge, EdgeSlab},
    node::{Node, NodeSlab},
    slab::{EntryId, Key as _, Slab},
    EdgeId, NodeId,
};

// TODO: move to node closure
// TODO: or just don't closure it
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Key {
    EndpointsToEdges(NodeId, NodeId),
}

#[derive(Debug, Clone, PartialEq)]
struct ClosureStorage {
    inner: HashMap<Key, RoaringBitmap, FnvBuildHasher>,
    nodes: RoaringBitmap,
}

impl ClosureStorage {
    fn new() -> Self {
        Self {
            inner: HashMap::with_hasher(FnvBuildHasher::default()),
            nodes: RoaringBitmap::new(),
        }
    }

    fn create_edge<T, U>(&mut self, edge: &Edge<T>, nodes: &mut NodeSlab<U>) {
        let raw_index = edge.id.into_id().raw();

        let source = edge.source;
        let target = edge.target;

        if let Some(source) = nodes.get_mut(source) {
            source.closures.insert_outgoing_node(target);
            source.closures.insert_outgoing_edge(edge.id);
        }

        if let Some(target) = nodes.get_mut(target) {
            target.closures.insert_incoming_node(source);
            target.closures.insert_incoming_edge(edge.id);
        }

        self.inner
            .entry(Key::EndpointsToEdges(source, target))
            .or_default()
            .add(raw_index);
    }

    fn remove_edge<T, U>(&mut self, edge: &Edge<T>, nodes: &mut NodeSlab<U>) {
        todo!()
        // let raw_index = edge.id.into_id().raw();
        //
        // let source = edge.source;
        // let target = edge.target;
        //
        // let is_multi = self
        //     .inner
        //     .get(&Key::EndpointsToEdges(edge.source, edge.target))
        //     .map_or(false, |bitmap| bitmap.cardinality() > 1);
        //
        // if let Some(source) = nodes.get_mut(source) {
        //     if !is_multi {
        //         source
        //             .closures
        //             .outgoing_nodes
        //             .remove(target.into_id().raw());
        //     }
        //
        //     source.closures.outgoing_edges.remove(raw_index);
        // }
        //
        // if let Some(target) = nodes.get_mut(target) {
        //     if !is_multi {
        //         target
        //             .closures
        //             .incoming_nodes
        //             .remove(source.into_id().raw());
        //     }
        //
        //     target.closures.incoming_edges.remove(raw_index);
        // }
        //
        // if let Some(edges) = self.inner.get_mut(&Key::EndpointsToEdges(source, target)) {
        //     edges.remove(raw_index);
        //
        //     if edges.is_empty() {
        //         self.inner.remove(&Key::EndpointsToEdges(source, target));
        //     }
        // }
    }

    fn remove_node<T>(&mut self, node: Node<T>, nodes: &mut NodeSlab<T>) -> (NodeId, T) {
        todo!()
        // let raw_index = node.id.into_id().raw();
        //
        // let targets = node.closures.outgoing_nodes;
        // for target in targets.iter() {
        //     let target_id = NodeId::from_id(EntryId::new_unchecked(target));
        //
        //     let Some(target) = nodes.get_mut(target_id) else {
        //         continue;
        //     };
        //
        //     target.closures.incoming_nodes.remove(raw_index);
        //
        //     self.inner
        //         .remove(&Key::EndpointsToEdges(node.id, target_id));
        // }
        //
        // let sources = node.closures.incoming_nodes;
        // for source in sources.iter() {
        //     let source_id = NodeId::from_id(EntryId::new_unchecked(source));
        //
        //     let Some(source) = nodes.get_mut(source_id) else {
        //         continue;
        //     };
        //
        //     source.closures.outgoing_nodes.remove(raw_index);
        //     self.inner
        //         .remove(&Key::EndpointsToEdges(source_id, node.id));
        // }
        //
        // (node.id, node.weight)
    }

    fn clear<N>(&mut self, nodes: &mut NodeSlab<N>) {
        self.inner.clear();

        for node in nodes.iter_mut() {
            node.closures.clear();
        }
    }

    fn refresh<N, E>(&mut self, nodes: &mut NodeSlab<N>, edges: &EdgeSlab<E>) {
        self.clear(nodes);

        for edge in edges.iter() {
            self.create_edge(edge, nodes);
        }
    }

    fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }
}

pub(crate) struct EdgeClosure<'a> {
    storage: &'a ClosureStorage,
}

impl<'a> EdgeClosure<'a> {
    const fn new(storage: &'a ClosureStorage) -> Self {
        Self { storage }
    }

    pub(crate) fn endpoints_to_edges(
        &self,
        source: NodeId,
        target: NodeId,
    ) -> impl Iterator<Item = EdgeId> + 'a {
        let Some(bitmap) = self
            .storage
            .inner
            .get(&Key::EndpointsToEdges(source, target))
        else {
            return Either::Left(core::iter::empty());
        };

        Either::Right(
            bitmap
                .iter()
                .map(|value| EdgeId::from_id(EntryId::new_unchecked(value))),
        )
    }

    pub(crate) fn undirected_endpoints_to_edges(
        &self,
        source: NodeId,
        target: NodeId,
    ) -> impl Iterator<Item = EdgeId> + 'a {
        let Some(source_to_targets) = self
            .storage
            .inner
            .get(&Key::EndpointsToEdges(source, target))
        else {
            return Either::Left(core::iter::empty());
        };

        let Some(target_to_sources) = self
            .storage
            .inner
            .get(&Key::EndpointsToEdges(target, source))
        else {
            return Either::Right(Either::Right(
                source_to_targets
                    .iter()
                    .map(|value| EdgeId::from_id(EntryId::new_unchecked(value))),
            ));
        };

        Either::Right(Either::Left(
            UnionIterator::new(source_to_targets, target_to_sources)
                .map(|value| EdgeId::from_id(EntryId::new_unchecked(value))),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Closures {
    storage: ClosureStorage,
}

impl Closures {
    pub(crate) fn new() -> Self {
        Self {
            storage: ClosureStorage::new(),
        }
    }

    pub(crate) fn remove_node<T>(&mut self, node: Node<T>, nodes: &mut NodeSlab<T>) -> (NodeId, T) {
        self.storage.remove_node(node, nodes)
    }

    pub(crate) const fn edges(&self) -> EdgeClosure<'_> {
        EdgeClosure::new(&self.storage)
    }

    pub(crate) fn create_edge<T, U>(&mut self, edge: &Edge<T>, nodes: &mut NodeSlab<U>) {
        self.storage.create_edge(edge, nodes);
    }

    pub(crate) fn remove_edge<T, U>(&mut self, edge: &Edge<T>, nodes: &mut NodeSlab<U>) {
        self.storage.remove_edge(edge, nodes);
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        self.storage.reserve(additional);
    }

    pub(crate) fn shrink_to_fit(&mut self) {
        self.storage.shrink_to_fit();
    }

    pub(crate) fn refresh<N, E>(
        &mut self,
        nodes: &mut Slab<NodeId, Node<N>>,
        edges: &Slab<EdgeId, Edge<E>>,
    ) {
        self.storage.refresh(nodes, edges);
    }

    pub(crate) fn clear<N>(&mut self, nodes: &mut NodeSlab<N>) {
        self.storage.clear(nodes);
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::iter::once;

    use hashbrown::{HashMap, HashSet};
    use petgraph_core::{attributes::Attributes, edge::marker::Directed, GraphDirectionality};
    use roaring::RoaringBitmap;

    use crate::{
        closure::Key,
        slab::{EntryId, Key as _},
        DinoGraph, DinoStorage, EdgeId, NodeId,
    };

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct EvaluatedNodeClosure {
        outgoing_neighbours: HashSet<NodeId>,
        incoming_neighbours: HashSet<NodeId>,

        neighbours: HashSet<NodeId>,

        outgoing_edges: HashSet<EdgeId>,
        incoming_edges: HashSet<EdgeId>,

        edges: HashSet<EdgeId>,
    }

    impl EvaluatedNodeClosure {
        fn new<N, E>(storage: &DinoStorage<N, E>, id: NodeId) -> Self {
            let node = storage.nodes.get(id).expect("node not found");

            Self {
                outgoing_neighbours: node.closures.outgoing_neighbours().collect(),
                incoming_neighbours: node.closures.incoming_neighbours().collect(),

                neighbours: node.closures.neighbours().collect(),

                outgoing_edges: node.closures.outgoing_edges().collect(),
                incoming_edges: node.closures.incoming_edges().collect(),

                edges: node.closures.edges().collect(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct EvaluatedEdgeClosures {
        source_to_targets: HashMap<NodeId, HashSet<NodeId>>,
        target_to_sources: HashMap<NodeId, HashSet<NodeId>>,

        source_to_edges: HashMap<NodeId, HashSet<EdgeId>>,
        targets_to_edges: HashMap<NodeId, HashSet<EdgeId>>,

        endpoints_to_edges: HashMap<(NodeId, NodeId), HashSet<EdgeId>>,
    }

    impl EvaluatedEdgeClosures {
        fn new<N, E>(storage: &DinoStorage<N, E>) -> Self {
            Self {
                source_to_targets: storage
                    .nodes
                    .entries()
                    .map(|(id, node)| (id, node.closures.outgoing_neighbours().collect()))
                    .collect(),
                target_to_sources: storage
                    .nodes
                    .entries()
                    .map(|(id, node)| (id, node.closures.incoming_neighbours().collect()))
                    .collect(),

                source_to_edges: storage
                    .nodes
                    .entries()
                    .map(|(id, node)| (id, node.closures.outgoing_edges().collect()))
                    .collect(),
                targets_to_edges: storage
                    .nodes
                    .entries()
                    .map(|(id, node)| (id, node.closures.incoming_edges().collect()))
                    .collect(),

                endpoints_to_edges: storage
                    .closures
                    .storage
                    .inner
                    .iter()
                    .filter_map(|(key, bitmap)| match key {
                        Key::EndpointsToEdges(source, target) => Some((
                            (*source, *target),
                            bitmap
                                .iter()
                                .map(|id| EdgeId::from_id(EntryId::new_unchecked(id)))
                                .collect::<HashSet<_>>(),
                        )),
                        _ => None,
                    })
                    .collect(),
            }
        }
    }

    macro_rules! map {
        (
            $(
                $key:expr => $value:expr
            ),*
            $(,)?
        ) => {{
            let mut map = ::hashbrown::HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }};
    }

    #[test]
    fn single_node() {
        let mut graph = DinoGraph::<u8, u8, Directed>::new();

        let node = graph.try_insert_node(1).unwrap();
        let id = *node.id();

        let closures = &graph.storage().closures;

        assert_eq!(isolated(&graph), once(id).collect());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), id),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: HashSet::new(),
                neighbours: HashSet::new(),
                outgoing_edges: HashSet::new(),
                incoming_edges: HashSet::new(),
                edges: HashSet::new(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    id => HashSet::new(),
                },
                target_to_sources: map! {
                    id => HashSet::new(),
                },
                source_to_edges: map! {
                    id => HashSet::new(),
                },
                targets_to_edges: map! {
                    id => HashSet::new(),
                },
                endpoints_to_edges: HashMap::new(),
            }
        );
    }

    fn isolated<N, E, D>(graph: &DinoGraph<N, E, D>) -> HashSet<NodeId>
    where
        D: GraphDirectionality,
    {
        graph
            .storage()
            .nodes
            .entries()
            .filter_map(|(id, node)| node.closures.is_isolated().then_some(id))
            .collect()
    }

    #[test]
    fn multiple_nodes() {
        let mut graph = DinoGraph::<u8, u8, Directed>::new();

        let a = graph.try_insert_node(Attributes::new(1)).unwrap();
        let a = *a.id();

        let b = graph.try_insert_node(Attributes::new(2)).unwrap();
        let b = *b.id();

        assert_eq!(isolated(&graph), [a, b].into_iter().collect());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: HashSet::new(),
                neighbours: HashSet::new(),
                outgoing_edges: HashSet::new(),
                incoming_edges: HashSet::new(),
                edges: HashSet::new(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), b),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: HashSet::new(),
                neighbours: HashSet::new(),
                outgoing_edges: HashSet::new(),
                incoming_edges: HashSet::new(),
                edges: HashSet::new(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => HashSet::new(),
                    b => HashSet::new(),
                },
                target_to_sources: map! {
                    a => HashSet::new(),
                    b => HashSet::new(),
                },
                source_to_edges: map! {
                    a => HashSet::new(),
                    b => HashSet::new(),
                },
                targets_to_edges: map! {
                    a => HashSet::new(),
                    b => HashSet::new(),
                },
                endpoints_to_edges: HashMap::new(),
            }
        );
    }

    #[test]
    fn connection() {
        let mut graph = DinoGraph::<u8, u8, Directed>::new();

        let a = graph.try_insert_node(1u8).unwrap();
        let a = *a.id();

        let b = graph.try_insert_node(1u8).unwrap();
        let b = *b.id();

        let edge = graph.try_insert_edge(1u8, &a, &b).unwrap();
        let edge = *edge.id();

        assert!(isolated(&graph).is_empty());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(b).collect(),
                incoming_neighbours: HashSet::new(),
                neighbours: once(b).collect(),
                outgoing_edges: once(edge).collect(),
                incoming_edges: HashSet::new(),
                edges: once(edge).collect(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), b),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: once(a).collect(),
                neighbours: once(a).collect(),
                outgoing_edges: HashSet::new(),
                incoming_edges: once(edge).collect(),
                edges: once(edge).collect(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => once(b).collect(),
                    b => HashSet::new(),
                },
                target_to_sources: map! {
                    b => once(a).collect(),
                    a => HashSet::new(),
                },
                source_to_edges: map! {
                    a => once(edge).collect(),
                    b => HashSet::new(),
                },
                targets_to_edges: map! {
                    b => once(edge).collect(),
                    a => HashSet::new(),
                },
                endpoints_to_edges: map! {
                    (a, b) => once(edge).collect(),
                }
            }
        );
    }

    #[test]
    fn self_loop() {
        let mut graph = DinoGraph::<u8, u8, Directed>::new();

        let a = graph.try_insert_node(1u8).unwrap();
        let a = *a.id();

        let edge = graph.try_insert_edge(1u8, &a, &a).unwrap();
        let edge = *edge.id();

        assert!(isolated(&graph).is_empty());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(a).collect(),
                incoming_neighbours: once(a).collect(),
                neighbours: once(a).collect(),
                outgoing_edges: once(edge).collect(),
                incoming_edges: once(edge).collect(),
                edges: once(edge).collect(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => once(a).collect(),
                },
                target_to_sources: map! {
                    a => once(a).collect(),
                },
                source_to_edges: map! {
                    a => once(edge).collect(),
                },
                targets_to_edges: map! {
                    a => once(edge).collect(),
                },
                endpoints_to_edges: map! {
                    (a, a) => once(edge).collect(),
                }
            }
        );
    }

    struct MultipleConnections {
        graph: DinoGraph<u8, u8, Directed>,

        a: NodeId,
        b: NodeId,
        c: NodeId,

        ab: EdgeId,
        bc: EdgeId,
        ca: EdgeId,
    }

    impl MultipleConnections {
        fn create() -> Self {
            let mut graph = DinoGraph::<u8, u8, Directed>::new();

            let a = graph.try_insert_node(1u8).unwrap();
            let a = *a.id();

            let b = graph.try_insert_node(1u8).unwrap();
            let b = *b.id();

            let c = graph.try_insert_node(1u8).unwrap();
            let c = *c.id();

            let ab = graph.try_insert_edge(1u8, &a, &b).unwrap();
            let ab = *ab.id();

            let bc = graph.try_insert_edge(1u8, &b, &c).unwrap();
            let bc = *bc.id();

            let ca = graph.try_insert_edge(1u8, &c, &a).unwrap();
            let ca = *ca.id();

            Self {
                graph,
                a,
                b,
                c,
                ab,
                bc,
                ca,
            }
        }

        fn assert(&self) {
            let Self {
                graph,
                a,
                b,
                c,
                ab,
                bc,
                ca,
            } = self;

            let (a, b, c, ab, bc, ca) = (*a, *b, *c, *ab, *bc, *ca);

            assert!(isolated(&graph).is_empty());

            assert_eq!(
                EvaluatedNodeClosure::new(graph.storage(), a),
                EvaluatedNodeClosure {
                    outgoing_neighbours: once(b).collect(),
                    incoming_neighbours: once(c).collect(),
                    neighbours: [b, c].into_iter().collect(),
                    outgoing_edges: once(ab).collect(),
                    incoming_edges: once(ca).collect(),
                    edges: [ab, ca].into_iter().collect(),
                }
            );

            assert_eq!(
                EvaluatedNodeClosure::new(graph.storage(), b),
                EvaluatedNodeClosure {
                    outgoing_neighbours: once(c).collect(),
                    incoming_neighbours: once(a).collect(),
                    neighbours: [c, a].into_iter().collect(),
                    outgoing_edges: once(bc).collect(),
                    incoming_edges: once(ab).collect(),
                    edges: [bc, ab].into_iter().collect(),
                }
            );

            assert_eq!(
                EvaluatedNodeClosure::new(graph.storage(), c),
                EvaluatedNodeClosure {
                    outgoing_neighbours: once(a).collect(),
                    incoming_neighbours: once(b).collect(),
                    neighbours: [a, b].into_iter().collect(),
                    outgoing_edges: once(ca).collect(),
                    incoming_edges: once(bc).collect(),
                    edges: [ca, bc].into_iter().collect(),
                }
            );

            assert_eq!(
                EvaluatedEdgeClosures::new(graph.storage()),
                EvaluatedEdgeClosures {
                    source_to_targets: map! {
                        a => once(b).collect(),
                        b => once(c).collect(),
                        c => once(a).collect(),
                    },
                    target_to_sources: map! {
                        a => once(c).collect(),
                        b => once(a).collect(),
                        c => once(b).collect(),
                    },
                    source_to_edges: map! {
                        a => once(ab).collect(),
                        b => once(bc).collect(),
                        c => once(ca).collect(),
                    },
                    targets_to_edges: map! {
                        a => once(ca).collect(),
                        b => once(ab).collect(),
                        c => once(bc).collect(),
                    },
                    endpoints_to_edges: map! {
                        (a, b) => once(ab).collect(),
                        (b, c) => once(bc).collect(),
                        (c, a) => once(ca).collect(),
                    },
                }
            );
        }
    }

    #[test]
    fn multiple_connections() {
        let graph = MultipleConnections::create();
        graph.assert();
    }

    #[test]
    fn multi_graph() {
        let mut graph = DinoGraph::<u8, u8, Directed>::new();

        let a = graph.try_insert_node(1u8).unwrap();
        let a = *a.id();

        let b = graph.try_insert_node(1u8).unwrap();
        let b = *b.id();

        let ab1 = graph.try_insert_edge(1u8, &a, &b).unwrap();
        let ab1 = *ab1.id();

        let ab2 = graph.try_insert_edge(1u8, &a, &b).unwrap();
        let ab2 = *ab2.id();

        assert!(isolated(&graph).is_empty());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(b).collect(),
                incoming_neighbours: HashSet::new(),
                neighbours: once(b).collect(),
                outgoing_edges: [ab1, ab2].into_iter().collect(),
                incoming_edges: HashSet::new(),
                edges: [ab1, ab2].into_iter().collect(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), b),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: once(a).collect(),
                neighbours: once(a).collect(),
                outgoing_edges: HashSet::new(),
                incoming_edges: [ab1, ab2].into_iter().collect(),
                edges: [ab1, ab2].into_iter().collect(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => once(b).collect(),
                    b => HashSet::new(),
                },
                target_to_sources: map! {
                    a => HashSet::new(),
                    b => once(a).collect(),
                },
                source_to_edges: map! {
                    a => [ab1, ab2].into_iter().collect(),
                    b => HashSet::new(),
                },
                targets_to_edges: map! {
                    a => HashSet::new(),
                    b => [ab1, ab2].into_iter().collect(),
                },
                endpoints_to_edges: map! {
                    (a, b) => [ab1, ab2].into_iter().collect(),
                },
            }
        );
    }

    #[test]
    fn remove_node() {
        let graph = MultipleConnections::create();
        graph.assert();

        let MultipleConnections {
            mut graph,
            a,
            b,
            c,
            ca,
            ..
        } = graph;

        graph.remove_node(&b).unwrap();

        assert!(isolated(&graph).is_empty());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: once(c).collect(),
                neighbours: once(c).collect(),
                outgoing_edges: HashSet::new(),
                incoming_edges: once(ca).collect(),
                edges: once(ca).collect(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), c),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(a).collect(),
                incoming_neighbours: HashSet::new(),
                neighbours: once(a).collect(),
                outgoing_edges: once(ca).collect(),
                incoming_edges: HashSet::new(),
                edges: once(ca).collect(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => HashSet::new(),
                    c => once(a).collect(),
                },
                target_to_sources: map! {
                    a => once(c).collect(),
                    c => HashSet::new(),
                },
                source_to_edges: map! {
                    a => HashSet::new(),
                    c => once(ca).collect(),
                },
                targets_to_edges: map! {
                    a => once(ca).collect(),
                    c => HashSet::new(),
                },
                endpoints_to_edges: map! {
                    (c, a) => once(ca).collect(),
                },
            }
        );
    }

    #[test]
    fn remove_edge() {
        let graph = MultipleConnections::create();
        graph.assert();

        let MultipleConnections {
            mut graph,
            a,
            b,
            c,
            ab,
            bc,
            ca,
        } = graph;

        graph.remove_edge(&bc).unwrap();

        assert!(isolated(&graph).is_empty());

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), a),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(b).collect(),
                incoming_neighbours: once(c).collect(),
                neighbours: [b, c].into_iter().collect(),
                outgoing_edges: once(ab).collect(),
                incoming_edges: once(ca).collect(),
                edges: [ab, ca].into_iter().collect(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), b),
            EvaluatedNodeClosure {
                outgoing_neighbours: HashSet::new(),
                incoming_neighbours: once(a).collect(),
                neighbours: once(a).collect(),
                outgoing_edges: HashSet::new(),
                incoming_edges: once(ab).collect(),
                edges: once(ab).collect(),
            }
        );

        assert_eq!(
            EvaluatedNodeClosure::new(graph.storage(), c),
            EvaluatedNodeClosure {
                outgoing_neighbours: once(a).collect(),
                incoming_neighbours: HashSet::new(),
                neighbours: once(a).collect(),
                outgoing_edges: once(ca).collect(),
                incoming_edges: HashSet::new(),
                edges: once(ca).collect(),
            }
        );

        assert_eq!(
            EvaluatedEdgeClosures::new(graph.storage()),
            EvaluatedEdgeClosures {
                source_to_targets: map! {
                    a => once(b).collect(),
                    b => HashSet::new(),
                    c => once(a).collect(),
                },
                target_to_sources: map! {
                    a => once(c).collect(),
                    b => once(a).collect(),
                    c => HashSet::new(),
                },
                source_to_edges: map! {
                    a => once(ab).collect(),
                    b => HashSet::new(),
                    c => once(ca).collect(),
                },
                targets_to_edges: map! {
                    a => once(ca).collect(),
                    b => once(ab).collect(),
                    c => HashSet::new(),
                },
                endpoints_to_edges: map! {
                    (a, b) => once(ab).collect(),
                    (c, a) => once(ca).collect(),
                },
            }
        );
    }
}
