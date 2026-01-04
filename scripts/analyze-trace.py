#!/usr/bin/env python3
"""
Analyze Chrome DevTools performance traces to find CPU hotspots.

Usage:
    python scripts/analyze-trace.py bench-trace/Trace-webgpu-260104-3.json
    python scripts/analyze-trace.py bench-trace/*.json --compare
"""

import json
import sys
import argparse
from collections import defaultdict
from pathlib import Path


def load_trace(path: str) -> dict:
    """Load a Chrome trace JSON file."""
    with open(path, 'r') as f:
        return json.load(f)


def extract_cpu_profile(trace: dict) -> tuple[dict, list, list]:
    """Extract CPU profile nodes and samples from trace."""
    nodes = {}
    samples = []
    time_deltas = []

    for event in trace.get('traceEvents', []):
        if event.get('name') != 'ProfileChunk':
            continue

        data = event.get('args', {}).get('data', {})
        cpu_profile = data.get('cpuProfile', {})

        for node in cpu_profile.get('nodes', []):
            node_id = node.get('id')
            call_frame = node.get('callFrame', {})
            nodes[node_id] = {
                'functionName': call_frame.get('functionName', '(unknown)'),
                'url': call_frame.get('url', ''),
                'lineNumber': call_frame.get('lineNumber'),
                'columnNumber': call_frame.get('columnNumber'),
                'codeType': call_frame.get('codeType', ''),
                'parent': node.get('parent'),
            }

        samples.extend(cpu_profile.get('samples', []))
        time_deltas.extend(data.get('timeDeltas', []))

    return nodes, samples, time_deltas


def get_full_stack(nodes: dict, node_id: int) -> list[tuple[int, str]]:
    """Get the full call stack for a node (leaf to root)."""
    stack = []
    current = node_id
    visited = set()
    while current and current not in visited:
        visited.add(current)
        node = nodes.get(current)
        if node:
            stack.append((current, node['functionName']))
            current = node.get('parent')
        else:
            break
    return stack


def analyze_samples(nodes: dict, samples: list, time_deltas: list) -> dict:
    """Analyze samples with both self and total time."""
    self_samples = defaultdict(int)
    self_time = defaultdict(int)
    total_samples = defaultdict(int)
    total_time = defaultdict(int)

    # Track parent-child relationships for call tree
    call_tree = defaultdict(lambda: defaultdict(int))  # parent -> child -> count

    for i, node_id in enumerate(samples):
        delta = time_deltas[i] if i < len(time_deltas) else 0
        stack = get_full_stack(nodes, node_id)

        if not stack:
            continue

        # Self time - only the leaf function
        leaf_id, leaf_name = stack[0]
        self_samples[leaf_name] += 1
        self_time[leaf_name] += delta

        # Total time - all functions in stack
        seen = set()
        for j, (nid, name) in enumerate(stack):
            if name not in seen:
                seen.add(name)
                total_samples[name] += 1
                total_time[name] += delta

            # Build call tree (child -> parent relationship)
            if j + 1 < len(stack):
                parent_name = stack[j + 1][1]
                call_tree[parent_name][name] += 1

    return {
        'self_samples': dict(self_samples),
        'self_time': dict(self_time),
        'total_samples': dict(total_samples),
        'total_time': dict(total_time),
        'call_tree': {k: dict(v) for k, v in call_tree.items()},
    }


def format_time(us: int) -> str:
    """Format microseconds as human-readable string."""
    if us >= 1_000_000:
        return f"{us / 1_000_000:.2f}s"
    elif us >= 1_000:
        return f"{us / 1_000:.2f}ms"
    else:
        return f"{us}µs"


def demangle_rust_name(name: str) -> str:
    """Simplify mangled Rust function names for readability."""
    import re
    # Remove hash suffixes like [40ee7ce29d520c24]
    name = re.sub(r'\[[0-9a-f]{16}\]', '', name)
    # Simplify common patterns
    name = name.replace('core::ops::function::', '')
    name = name.replace('alloc::vec::Vec<', 'Vec<')
    name = name.replace('alloc::boxed::Box<', 'Box<')
    return name


def truncate(s: str, max_len: int) -> str:
    """Truncate string with ellipsis if too long."""
    if len(s) <= max_len:
        return s
    return s[:max_len-3] + '...'


def print_call_tree(analysis: dict, total_sample_count: int, total_time_us: int):
    """Print a hierarchical call tree showing cumulative time."""
    total_samples = analysis['total_samples']
    total_time = analysis['total_time']
    self_samples = analysis['self_samples']
    self_time = analysis['self_time']
    call_tree = analysis['call_tree']

    print(f"\n{'='*100}")
    print("CALL TREE (Cumulative Time - shows where time is spent INCLUDING children)")
    print(f"{'='*100}")
    print(f"{'Self':>8} {'Total':>8} {'Self%':>6} {'Total%':>6}  Function")
    print(f"{'─'*8} {'─'*8} {'─'*6} {'─'*6}  {'─'*70}")

    # Sort by total time
    sorted_funcs = sorted(
        [(k, v) for k, v in total_time.items()
         if k not in ('(idle)', '(program)', '(root)')],
        key=lambda x: -x[1]
    )

    def print_subtree(name: str, indent: int = 0, printed: set = None, max_depth: int = 8):
        if printed is None:
            printed = set()
        if name in printed or indent > max_depth:
            return
        printed.add(name)

        self_t = self_time.get(name, 0)
        total_t = total_time.get(name, 0)
        self_pct = (self_t / total_time_us * 100) if total_time_us else 0
        total_pct = (total_t / total_time_us * 100) if total_time_us else 0

        # Skip tiny contributions
        if total_pct < 0.1 and indent > 0:
            return

        prefix = "  " * indent + ("└─ " if indent > 0 else "")
        display_name = truncate(demangle_rust_name(name), 70 - len(prefix))

        print(f"{format_time(self_t):>8} {format_time(total_t):>8} {self_pct:>5.1f}% {total_pct:>5.1f}%  {prefix}{display_name}")

        # Print children sorted by their contribution
        children = call_tree.get(name, {})
        sorted_children = sorted(children.items(), key=lambda x: -total_time.get(x[0], 0))

        for child_name, _ in sorted_children[:5]:  # Limit to top 5 children
            if total_time.get(child_name, 0) / total_time_us * 100 >= 0.1:
                print_subtree(child_name, indent + 1, printed, max_depth)

    # Start from top-level functions
    printed = set()
    for name, _ in sorted_funcs[:30]:
        if name not in printed:
            print_subtree(name, 0, printed)


def print_flat_profile(analysis: dict, total_sample_count: int, total_time_us: int):
    """Print flat profile sorted by self time."""
    self_samples = analysis['self_samples']
    self_time = analysis['self_time']
    total_time = analysis['total_time']

    print(f"\n{'='*100}")
    print("FLAT PROFILE (Self Time - where CPU was directly executing, NOT in children)")
    print(f"{'='*100}")
    print(f"{'Self':>10} {'Total':>10} {'Self%':>7} {'Total%':>7}  Function")
    print(f"{'─'*10} {'─'*10} {'─'*7} {'─'*7}  {'─'*60}")

    sorted_funcs = sorted(
        [(k, v) for k, v in self_time.items()
         if k not in ('(idle)', '(program)', '(root)', '(garbage collector)')],
        key=lambda x: -x[1]
    )

    for name, self_t in sorted_funcs[:40]:
        total_t = total_time.get(name, 0)
        self_pct = (self_t / total_time_us * 100) if total_time_us else 0
        total_pct = (total_t / total_time_us * 100) if total_time_us else 0
        display_name = truncate(demangle_rust_name(name), 60)
        print(f"{format_time(self_t):>10} {format_time(total_t):>10} {self_pct:>6.2f}% {total_pct:>6.2f}%  {display_name}")


def print_bottleneck_analysis(analysis: dict, total_time_us: int):
    """Identify and explain specific performance bottlenecks."""
    total_time = analysis['total_time']
    self_time = analysis['self_time']

    print(f"\n{'='*100}")
    print("BOTTLENECK ANALYSIS")
    print(f"{'='*100}")

    categories = defaultdict(list)

    for name, t in total_time.items():
        pct = (t / total_time_us * 100) if total_time_us else 0
        self_t = self_time.get(name, 0)
        self_pct = (self_t / total_time_us * 100) if total_time_us else 0

        if pct < 0.5:  # Skip tiny contributions
            continue

        if 'new_from_slice' in name:
            categories['WASM→JS Memory Copy'].append((name, t, pct, self_pct))
        elif 'write_buffer' in name.lower():
            categories['GPU Buffer Upload'].append((name, t, pct, self_pct))
        elif 'sync_dirty' in name.lower():
            categories['Dirty Buffer Sync'].append((name, t, pct, self_pct))
        elif 'createBindGroup' in name:
            categories['Bind Group Creation'].append((name, t, pct, self_pct))
        elif 'createCommandEncoder' in name:
            categories['Command Encoder Creation'].append((name, t, pct, self_pct))
        elif 'wasm-to-js' in name.lower():
            categories['WASM→JS Call'].append((name, t, pct, self_pct))
        elif 'js-to-wasm' in name.lower():
            categories['JS→WASM Call'].append((name, t, pct, self_pct))
        elif 'submit' in name.lower() and 'wbg' in name:
            categories['GPU Submit'].append((name, t, pct, self_pct))
        elif 'naga' in name.lower():
            categories['Shader Compilation (Naga)'].append((name, t, pct, self_pct))

    if not categories:
        print("  No specific bottleneck patterns detected.")
        return

    for category, items in sorted(categories.items(), key=lambda x: -sum(i[2] for i in x[1])):
        total_pct = sum(i[2] for i in items)
        print(f"\n  [{category}] - {total_pct:.1f}% total")
        for name, t, pct, self_pct in sorted(items, key=lambda x: -x[2])[:3]:
            display = truncate(demangle_rust_name(name), 60)
            print(f"    {format_time(t):>10} ({pct:.1f}% total, {self_pct:.1f}% self) {display}")


def print_context(analysis: dict, total_sample_count: int, total_time_us: int):
    """Print idle/overhead context."""
    self_samples = analysis['self_samples']
    self_time = analysis['self_time']

    print(f"\n{'─'*100}")
    print("CONTEXT (Idle/Overhead)")
    print(f"{'─'*100}")

    for name in ['(idle)', '(program)', '(garbage collector)']:
        if name in self_time:
            t = self_time[name]
            pct = (t / total_time_us * 100) if total_time_us else 0
            print(f"  {format_time(t):>10} {pct:>6.1f}%  {name}")


def print_report(trace_path: str, nodes: dict, samples: list, time_deltas: list):
    """Print comprehensive CPU profile report."""
    analysis = analyze_samples(nodes, samples, time_deltas)

    total_sample_count = len(samples)
    total_time_us = sum(time_deltas) if time_deltas else 0

    # Calculate active time (excluding idle/program/root)
    idle_names = {'(idle)', '(program)', '(root)'}
    idle_time_us = analysis['self_time'].get('(idle)', 0) + \
                   analysis['self_time'].get('(program)', 0) + \
                   analysis['self_time'].get('(root)', 0)
    active_time_us = total_time_us - idle_time_us

    print(f"\n{'#'*100}")
    print(f"# CPU Profile Analysis: {Path(trace_path).name}")
    print(f"{'#'*100}")
    print(f"Total samples: {total_sample_count:,}")
    print(f"Total time: {format_time(total_time_us)}")
    print(f"Active time: {format_time(active_time_us)} ({active_time_us/total_time_us*100:.1f}% - excludes idle)")
    print(f"Sample interval: ~{total_time_us // max(total_sample_count, 1)}µs")
    print()
    print("NOTE: Percentages below are relative to ACTIVE time (matching Chrome DevTools)")

    # Print all sections using active_time for percentages (like Chrome DevTools)
    print_bottleneck_analysis(analysis, active_time_us)
    print_call_tree(analysis, total_sample_count, active_time_us)
    print_flat_profile(analysis, total_sample_count, active_time_us)
    print_context(analysis, total_sample_count, total_time_us)


def main():
    parser = argparse.ArgumentParser(description='Analyze Chrome DevTools performance traces')
    parser.add_argument('traces', nargs='+', help='Trace JSON files to analyze')
    parser.add_argument('--top', type=int, default=40, help='Number of top functions to show')
    args = parser.parse_args()

    for trace_path in args.traces:
        try:
            trace = load_trace(trace_path)
            nodes, samples, time_deltas = extract_cpu_profile(trace)

            if not samples:
                print(f"Warning: No CPU profile samples found in {trace_path}")
                continue

            print_report(trace_path, nodes, samples, time_deltas)

        except Exception as e:
            print(f"Error processing {trace_path}: {e}", file=sys.stderr)
            raise


if __name__ == '__main__':
    main()
