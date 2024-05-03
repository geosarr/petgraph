use crate::visit::{IntoNeighborsDirected, NodeCount, NodeIndexable};

use super::{Direction, UnitMeasure};

// To compute square root of float-pointing numbers.
pub trait Sqrt {
    fn sqrt(&self) -> Self;
}
impl Sqrt for f32 {
    fn sqrt(&self) -> Self {
        Self::sqrt(*self)
    }
}
impl Sqrt for f64 {
    fn sqrt(&self) -> Self {
        Self::sqrt(*self)
    }
}

pub fn hits<N, H>(network: N, tol: Option<H>, nb_iter: usize) -> (Vec<H>, Vec<H>)
where
    N: NodeCount + IntoNeighborsDirected + NodeIndexable,
    H: UnitMeasure + std::iter::Sum<H> + Copy + Sqrt,
{
    let node_count = network.node_count();
    if node_count == 0 {
        return (vec![], vec![]);
    }
    let mut tolerance = H::default_tol();
    if let Some(_tol) = tol {
        tolerance = _tol;
    }
    let nodeix = |i| network.from_index(i);
    let mut auth = vec![H::one(); node_count];
    let mut hub = vec![H::one(); node_count];

    for _ in 0..nb_iter {
        // Update the Authority scores
        let norm_sum_in_hubs = (0..node_count)
            .map(|page| {
                auth[page] = network
                    .neighbors_directed(nodeix(page), Direction::Incoming)
                    .map(|ix| hub[network.to_index(ix)])
                    .sum::<H>();
                auth[page] * auth[page]
            })
            .sum::<H>()
            .sqrt();

        // Update the Hub scores
        let norm_sum_out_auths = (0..node_count)
            .map(|page| {
                hub[page] = network
                    .neighbors_directed(nodeix(page), Direction::Outgoing)
                    .map(|ix| auth[network.to_index(ix)])
                    .sum::<H>();
                hub[page] * hub[page]
            })
            .sum::<H>()
            .sqrt();

        let new_auth = auth
            .iter()
            .map(|a| *a / norm_sum_in_hubs)
            .collect::<Vec<H>>();
        let new_hub = hub
            .iter()
            .map(|h| *h / norm_sum_out_auths)
            .collect::<Vec<H>>();

        let delta_auth = new_auth
            .iter()
            .zip(&auth)
            .map(|(new, old)| (*new - *old) * (*new - *old))
            .sum::<H>();
        let delta_hub = new_hub
            .iter()
            .zip(&hub)
            .map(|(new, old)| (*new - *old) * (*new - *old))
            .sum::<H>();
        let max_delta = if delta_auth < delta_hub {
            delta_hub
        } else {
            delta_auth
        };
        if max_delta <= tolerance {
            return (auth, hub);
        } else {
            auth = new_auth;
            hub = new_hub;
        }
    }
    return (auth, hub);
}

mod tests {
    use super::hits;
    use crate::Graph;
    #[test]
    fn test_code() {
        let mut graph = Graph::<usize, u16>::new();
        graph.add_node(0);
        graph.add_node(1);
        graph.add_node(2);
        graph.add_node(3);
        graph.extend_with_edges(&[(0, 1, 3), (0, 2, 2), (1, 2, 5), (1, 3, 2), (2, 3, 3)]);
        let (auth, hub) = hits(&graph, Some(0.0000001f32), 1000);
        println!("{:?}", auth);
        println!("{:?}", hub);
    }
}
