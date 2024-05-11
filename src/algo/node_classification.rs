use crate::data::DataMap;
use crate::graph::IndexType;
use crate::matrix_graph::Nullable;
use crate::visit::{IntoNeighbors, IntoNodeReferences, NodeCount, NodeRef};
use std::collections::HashMap;
use std::fmt::Debug;

pub fn label_propagation<G>(
    graph: G,
    labels: &[G::NodeWeight],
    nb_iter: usize,
) -> HashMap<G::NodeId, G::NodeWeight>
where
    G: IntoNodeReferences + NodeCount + IntoNeighbors + DataMap,
    G::NodeId: IndexType,
    G::NodeWeight: PartialEq + Clone + Debug + Nullable,
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
                for neighbor in graph.neighbors(node.id()) {
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
