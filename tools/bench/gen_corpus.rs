// Standalone corpus generator for M0 scale probes.
// Compile: rustc -O -o gen_corpus gen_corpus.rs
// Usage:   gen_corpus <out_dir> <count> <flat|fanout>
//
// Writes <count> realistic problem entity files (YAML frontmatter + body,
// ~500 bytes each) under <out_dir>/problems/. `flat` = one directory;
// `fanout` = problems/{ab}/{cd}/{id}.md sharded by the leading hex of the id.
// Ids are deterministic so probes can reference specific entities.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: gen_corpus <out_dir> <count> <flat|fanout>");
        std::process::exit(2);
    }
    let out_dir = PathBuf::from(&args[1]);
    let count: usize = args[2].parse().expect("count must be an integer");
    let fanout = match args[3].as_str() {
        "fanout" => true,
        "flat" => false,
        other => {
            eprintln!("layout must be 'flat' or 'fanout', got {other}");
            std::process::exit(2);
        }
    };

    let body = "This problem concerns a sub-component of the larger investigation. \
It requires careful analysis of the available evidence and a conjecture that can be \
subjected to criticism. Linked changes hold the experimental artifacts and data.";

    let problems = out_dir.join("problems");
    let mut made: HashSet<PathBuf> = HashSet::new();
    if !fanout {
        fs::create_dir_all(&problems).unwrap();
        made.insert(problems.clone());
    }

    for i in 0..count {
        // Deterministic pseudo-uuid7-shaped id.
        let id = format!("0195{:08x}-{:04x}-7def-8c3a-{:012x}", i, i % 0xffff, i);
        let dir = if fanout {
            let d = problems.join(&id[0..2]).join(&id[2..4]);
            if made.insert(d.clone()) {
                fs::create_dir_all(&d).unwrap();
            }
            d
        } else {
            problems.clone()
        };
        let content = format!(
            "---\n\
id: {id}\n\
title: \"Investigate facet {i} of the problem space\"\n\
status: open\n\
priority: medium\n\
created_at: 2026-06-19T12:00:00Z\n\
updated_at: 2026-06-19T12:00:00Z\n\
tags:\n- area:facet{facet}\n- size:M\n\
---\n\n{body}\n",
            id = id,
            i = i,
            facet = i % 50,
            body = body
        );
        let mut f = fs::File::create(dir.join(format!("{id}.md"))).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }
}
