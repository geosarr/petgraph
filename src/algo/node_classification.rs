use crate::data::DataMap;
use crate::matrix_graph::Nullable;
use crate::visit::{IntoNeighbors, IntoNodeReferences, NodeCount, NodeRef};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// Finds for a given node N, the neighborhood collecting the nodes that are far from N by at most k nodes.
fn k_neighborhood<G>(graph: G, node: G::NodeId, k: usize) -> Vec<G::NodeId>
where
    G: IntoNeighbors,
{
    if k == 0 {
        return vec![];
    } else if k == 1 {
        return graph.neighbors(node).collect();
    } else {
        let mut neighbor_nodes = graph.neighbors(node).collect::<Vec<G::NodeId>>();
        let mut collector = Vec::new();
        for node_id in neighbor_nodes.iter() {
            collector.extend(k_neighborhood(graph, *node_id, k - 1));
        }
        neighbor_nodes.extend(collector);
        return neighbor_nodes;
    }
}

pub fn label_propagation<G>(
    graph: G,
    labels: &[G::NodeWeight],
    k: usize,
    nb_iter: usize,
) -> HashMap<G::NodeId, G::NodeWeight>
where
    G: IntoNodeReferences + NodeCount + IntoNeighbors + DataMap,
    G::NodeId: Hash + Eq,
    G::NodeWeight: PartialEq + Clone + Nullable,
{
    let mut predicted_labels = HashMap::new();
    if graph.node_count() == 0 || labels.is_empty() {
        return predicted_labels;
    }
    let mut iter = 0;
    while iter < nb_iter {
        for node in graph.node_references() {
            // Ignore nodes with label.
            if predicted_labels.contains_key(&node.id()) || node.weight().is_null() {
                let mut label_frequencies = labels
                    .iter()
                    .map(|label| LabelFreq::new(label.clone(), 0))
                    .collect::<Vec<LabelFreq<G::NodeWeight>>>();
                // Find the most frequent label in the neighbourhood of the current node.
                for neighbor in k_neighborhood(graph, node.id(), k) {
                    let neighbor_label = graph.node_weight(neighbor).unwrap();
                    label_frequencies.iter_mut().for_each(|labelf| {
                        labelf.freq += usize::from(
                            (labelf.label == neighbor_label.clone()) && !neighbor_label.is_null(),
                        )
                    });
                }
                label_frequencies.sort();
                // Propagate the most frequent label if any.
                let most_frequent = label_frequencies.last().unwrap();
                if most_frequent.freq > 0 {
                    predicted_labels.insert(node.id(), most_frequent.label.clone());
                }
            }
        }
        iter += 1;
    }
    predicted_labels
}

/// To compare node labels by their frequencies.
#[derive(Debug)]
struct LabelFreq<L> {
    label: L,
    freq: usize,
}

impl<L> LabelFreq<L> {
    pub fn new(label: L, freq: usize) -> Self {
        Self { label, freq }
    }
}

impl<L> PartialEq for LabelFreq<L> {
    fn eq(&self, other: &Self) -> bool {
        self.freq.eq(&other.freq)
    }
}
impl<L> Eq for LabelFreq<L> {}
impl<L> PartialOrd for LabelFreq<L> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.freq.partial_cmp(&other.freq)
    }
}
impl<L> Ord for LabelFreq<L> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.freq.cmp(&other.freq)
    }
}
