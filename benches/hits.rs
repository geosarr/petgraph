#![feature(test)]
extern crate petgraph;
extern crate test;

use test::Bencher;

use petgraph::algo::hits;

#[allow(dead_code)]
mod common;

use common::directed_fan;

#[bench]
fn hits_bench(bench: &mut Bencher) {
    static NODE_COUNT: usize = 1_000;
    let g = directed_fan(NODE_COUNT);
    bench.iter(|| {
        let _scores = hits::<_, f32>(&g, None, 100, Default::default());
    });
}
