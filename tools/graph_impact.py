"""Change impact for a Rust item, from graphify's graph plus the exact use sites.

graphify's Rust extraction links `Type::function` uses (calls from other files, or the function passed as a value)
to the type, not the function. So this finds the use sites by text in the files the graph links to the type (or in
every graph file with --all-files), maps each site to the function node that encloses it, and then walks the
graph's reverse `calls` edges to the functions and tests that reach those sites.

Usage: python -I tools/graph_impact.py <graph.json> <repo root> <text to find> [--type <TypeLabel>] [--depth N]
"""
import argparse
import bisect
import collections
import json
import os
import re

parser = argparse.ArgumentParser()
parser.add_argument("graph")
parser.add_argument("root")
parser.add_argument("text")
parser.add_argument("--type", dest="type_label")
parser.add_argument("--depth", type=int, default=4)
args = parser.parse_args()

g = json.load(open(args.graph, encoding="utf-8"))
nodes = {n["id"]: n for n in g["nodes"]}
reverse_calls = collections.defaultdict(set)
for link in g["links"]:
    if link["relation"] == "calls":
        reverse_calls[link["target"]].add(link["source"])

# Candidate files: those with an edge to the type, or every file in the graph.
files = set()
if args.type_label:
    type_ids = {n["id"] for n in g["nodes"] if n["label"] == args.type_label}
    for link in g["links"]:
        for end, other in ((link["target"], link["source"]), (link["source"], link["target"])):
            if end in type_ids:
                source = nodes[other].get("source_file")
                if source:
                    files.add(source)
    for t in type_ids:
        if nodes[t].get("source_file"):
            files.add(nodes[t]["source_file"])
else:
    files = {n["source_file"] for n in g["nodes"] if n.get("source_file")}

# Function-like nodes per file, by start line, to find the one enclosing a site.
by_file = collections.defaultdict(list)
for n in g["nodes"]:
    loc = n.get("source_location") or ""
    if n.get("source_file") and loc.startswith("L") and n["label"].endswith("()"):
        by_file[n["source_file"]].append((int(loc[1:]), n["id"]))
for entries in by_file.values():
    entries.sort()

sites = []
pattern = re.compile(re.escape(args.text))
for source in sorted(files):
    path = os.path.join(args.root, source)
    if not os.path.isfile(path):
        continue
    for number, line in enumerate(open(path, encoding="utf-8", errors="replace"), 1):
        if pattern.search(line) and not line.lstrip().startswith("//"):
            entries = by_file.get(source, [])
            index = bisect.bisect_right(entries, (number, "￿")) - 1
            enclosing = entries[index][1] if index >= 0 else None
            sites.append((source, number, enclosing, line.strip()))

print(f"{len(sites)} sites of {args.text!r} in {len(files)} candidate files")
seeds = set()
for source, number, enclosing, text in sites:
    label = nodes[enclosing]["label"] if enclosing else "(file level)"
    print(f"  {source}:{number} in {label}: {text[:110]}")
    if enclosing:
        seeds.add(enclosing)

# Reverse call closure from the enclosing functions.
reached = {s: 0 for s in seeds}
frontier = set(seeds)
for depth in range(1, args.depth + 1):
    nxt = set()
    for node in frontier:
        for caller in reverse_calls[node]:
            if caller not in reached:
                reached[caller] = depth
                nxt.add(caller)
    frontier = nxt

tests = sorted(n for n in reached if "/tests/" in (nodes[n].get("source_file") or ""))
production = sorted(n for n in reached if n not in seeds and n not in tests)
print(f"\nproduction callers within {args.depth} hops: {len(production)}")
for n in production:
    print(f"  [{reached[n]}] {nodes[n]['label']} {nodes[n].get('source_file')}:{nodes[n].get('source_location')}")
print(f"\ntests reaching a site within {args.depth} hops: {len(tests)}")
by_test_file = collections.defaultdict(list)
for n in tests:
    by_test_file[nodes[n]["source_file"]].append(nodes[n]["label"])
for source, labels in sorted(by_test_file.items()):
    print(f"  {source}: {', '.join(sorted(labels))}")
