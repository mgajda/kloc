use std::time::Instant;
use tree_sitter::Parser;

fn child_scan(source: &[u8]) -> f64 {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter::Language::new(tree_sitter_rust::LANGUAGE)).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();
    let n = root.child_count() as u32;
    let t0 = Instant::now();
    let mut sink = 0u64;
    for i in (0..n).rev() {
        if let Some(c) = root.child(i) {
            sink = sink.wrapping_add(c.start_byte() as u64);
        }
    }
    let elapsed = t0.elapsed().as_secs_f64();
    println!("child-scan  root={:>8} children -> {:>10.3} s  (n^2 proxy {:.3e}) sink={sink}", n, elapsed, (n as f64).powi(2));
    elapsed
}

fn cursor_iter(source: &[u8]) -> f64 {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter::Language::new(tree_sitter_rust::LANGUAGE)).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();
    let t0 = Instant::now();
    let mut sink = 0u64;
    let mut count = 0u64;
    let mut c = root.walk();
    if c.goto_first_child() {
        loop {
            sink = sink.wrapping_add(c.node().start_byte() as u64);
            count += 1;
            if !c.goto_next_sibling() { break; }
        }
    }
    let elapsed = t0.elapsed().as_secs_f64();
    println!("cursor-iter  root children -> {count:>8} -> {:>10.3} s sink={sink}", elapsed);
    elapsed
}

fn main() {
    for n in [10_000u32, 20_000, 40_000, 80_000] {
        let mut src = String::new();
        for i in 0..n {
            src.push_str(&format!("fn f{i}() -> u64 {{ {i} }}\n"));
        }
        let bytes = src.into_bytes();
        let _ = child_scan(&bytes);
        let _ = cursor_iter(&bytes);
        println!();
    }
}
