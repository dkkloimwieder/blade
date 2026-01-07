#!/usr/bin/env python3
"""
Analyze WebGPU Inspector HTML recordings.

Usage:
    python3 scripts/analyze-webgpu-inspector.py <recording.html>
    python3 scripts/analyze-webgpu-inspector.py bench-trace/webgpu_record-culled-1.html
"""

import sys
import re
from collections import defaultdict
from pathlib import Path

def parse_webgpu_recording(html_path: str) -> dict:
    """Parse a WebGPU Inspector HTML recording and extract GPU commands."""

    with open(html_path, 'r') as f:
        content = f.read()

    results = {
        'draws': [],
        'dispatches': [],
        'buffers': [],
        'pipelines': [],
        'frame_count': 0,
        'summary': {}
    }

    # Count frames (D_F<n>_<m> pattern)
    frame_matches = re.findall(r'D_F(\d+)_\d+', content)
    if frame_matches:
        results['frame_count'] = max(int(f) for f in frame_matches) + 1

    # Extract draw calls: .draw(vertexCount, instanceCount, firstVertex, firstInstance)
    draw_pattern = r'\.draw\((\d+),(\d+),(\d+),(\d+)\)'
    for match in re.finditer(draw_pattern, content):
        results['draws'].append({
            'vertex_count': int(match.group(1)),
            'instance_count': int(match.group(2)),
            'first_vertex': int(match.group(3)),
            'first_instance': int(match.group(4)),
        })

    # Extract drawIndirect calls
    draw_indirect_pattern = r'\.drawIndirect\([^,]+,\s*(\d+)\)'
    for match in re.finditer(draw_indirect_pattern, content):
        results['draws'].append({
            'type': 'indirect',
            'offset': int(match.group(1)),
        })

    # Extract dispatch calls: .dispatchWorkgroups(x, y, z)
    dispatch_pattern = r'\.dispatchWorkgroups\((\d+),(\d+),(\d+)\)'
    for match in re.finditer(dispatch_pattern, content):
        results['dispatches'].append({
            'x': int(match.group(1)),
            'y': int(match.group(2)),
            'z': int(match.group(3)),
            'total_workgroups': int(match.group(1)) * int(match.group(2)) * int(match.group(3)),
        })

    # Extract buffer creations: createBuffer({...size:N...})
    buffer_pattern = r'createBuffer\(\{[^}]*"size":(\d+)[^}]*"label":"([^"]*)"'
    for match in re.finditer(buffer_pattern, content):
        results['buffers'].append({
            'size': int(match.group(1)),
            'label': match.group(2),
        })

    # Also try alternate order (label before size)
    buffer_pattern2 = r'createBuffer\(\{[^}]*"label":"([^"]*)"[^}]*"size":(\d+)'
    for match in re.finditer(buffer_pattern2, content):
        results['buffers'].append({
            'size': int(match.group(2)),
            'label': match.group(1),
        })

    # Extract pipeline labels
    pipeline_pattern = r'create(Compute|Render)Pipeline\(\{[^}]*"label":"([^"]*)"'
    for match in re.finditer(pipeline_pattern, content):
        results['pipelines'].append({
            'type': match.group(1).lower(),
            'label': match.group(2),
        })

    # Compute summary
    results['summary'] = compute_summary(results)

    return results


def compute_summary(results: dict) -> dict:
    """Compute summary statistics from parsed results."""

    summary = {}

    # Draw call analysis
    if results['draws']:
        direct_draws = [d for d in results['draws'] if 'type' not in d]
        indirect_draws = [d for d in results['draws'] if d.get('type') == 'indirect']

        summary['draw_calls'] = len(results['draws'])
        summary['direct_draws'] = len(direct_draws)
        summary['indirect_draws'] = len(indirect_draws)

        if direct_draws:
            total_vertices = sum(d['vertex_count'] * d['instance_count'] for d in direct_draws)
            total_instances = sum(d['instance_count'] for d in direct_draws)
            summary['total_vertices_drawn'] = total_vertices
            summary['total_instances'] = total_instances

            # Per-frame if we know frame count
            if results['frame_count'] > 0:
                summary['draws_per_frame'] = len(direct_draws) / results['frame_count']
                summary['instances_per_frame'] = total_instances / results['frame_count']

    # Dispatch analysis
    if results['dispatches']:
        summary['dispatch_calls'] = len(results['dispatches'])

        # Group by workgroup count
        dispatch_types = defaultdict(int)
        for d in results['dispatches']:
            key = f"{d['x']}x{d['y']}x{d['z']}"
            dispatch_types[key] += 1
        summary['dispatch_types'] = dict(dispatch_types)

        if results['frame_count'] > 0:
            summary['dispatches_per_frame'] = len(results['dispatches']) / results['frame_count']

    # Buffer analysis
    if results['buffers']:
        # Deduplicate by label
        unique_buffers = {}
        for b in results['buffers']:
            if b['label'] not in unique_buffers:
                unique_buffers[b['label']] = b['size']

        summary['buffer_count'] = len(unique_buffers)
        summary['total_buffer_memory'] = sum(unique_buffers.values())
        summary['buffers'] = unique_buffers

    return summary


def print_report(results: dict, html_path: str):
    """Print a formatted analysis report."""

    print("=" * 70)
    print(f"WebGPU Inspector Analysis: {Path(html_path).name}")
    print("=" * 70)

    summary = results['summary']

    print(f"\n📊 OVERVIEW")
    print("-" * 50)
    print(f"  Frames captured: {results['frame_count']}")
    print(f"  Draw calls: {summary.get('draw_calls', 0)}")
    print(f"  Dispatch calls: {summary.get('dispatch_calls', 0)}")

    if 'draws_per_frame' in summary:
        print(f"  Draws/frame: {summary['draws_per_frame']:.1f}")
    if 'dispatches_per_frame' in summary:
        print(f"  Dispatches/frame: {summary['dispatches_per_frame']:.1f}")

    print(f"\n🎨 DRAW CALLS")
    print("-" * 50)
    if results['draws']:
        direct_draws = [d for d in results['draws'] if 'type' not in d]
        indirect_draws = [d for d in results['draws'] if d.get('type') == 'indirect']

        print(f"  Direct draws: {len(direct_draws)}")
        print(f"  Indirect draws: {len(indirect_draws)}")

        if direct_draws:
            # Show unique draw configurations
            draw_configs = defaultdict(int)
            for d in direct_draws:
                key = f"vertices={d['vertex_count']}, instances={d['instance_count']}"
                draw_configs[key] += 1

            print(f"\n  Draw configurations:")
            for config, count in sorted(draw_configs.items(), key=lambda x: -x[1]):
                print(f"    {count}x: {config}")

            print(f"\n  Total instances drawn: {summary.get('total_instances', 0):,}")
            print(f"  Total vertices processed: {summary.get('total_vertices_drawn', 0):,}")

    print(f"\n⚡ COMPUTE DISPATCHES")
    print("-" * 50)
    if 'dispatch_types' in summary:
        for config, count in sorted(summary['dispatch_types'].items(), key=lambda x: -x[1]):
            # Parse workgroup size
            parts = config.split('x')
            total_wg = int(parts[0]) * int(parts[1]) * int(parts[2])
            threads = total_wg * 256  # Assuming workgroup_size(256)
            print(f"  {count}x: dispatchWorkgroups({config}) = {total_wg} workgroups (~{threads:,} threads)")

    print(f"\n💾 BUFFERS")
    print("-" * 50)
    if 'buffers' in summary:
        total_mb = summary['total_buffer_memory'] / (1024 * 1024)
        print(f"  Total GPU memory: {total_mb:.2f} MB")
        print(f"\n  Buffer breakdown:")
        for label, size in sorted(summary['buffers'].items(), key=lambda x: -x[1]):
            if size >= 1024 * 1024:
                size_str = f"{size / (1024*1024):.2f} MB"
            elif size >= 1024:
                size_str = f"{size / 1024:.1f} KB"
            else:
                size_str = f"{size} B"
            print(f"    {label}: {size_str}")

    # Performance insights
    print(f"\n💡 INSIGHTS")
    print("-" * 50)

    if results['draws']:
        direct_draws = [d for d in results['draws'] if 'type' not in d]
        if direct_draws and all(d['instance_count'] == direct_draws[0]['instance_count'] for d in direct_draws):
            instance_count = direct_draws[0]['instance_count']
            print(f"  ⚠️  All draws use {instance_count} instances (no GPU culling benefit)")
            print(f"      Consider: indirect draw with dynamic instance count")

    if 'dispatch_types' in summary:
        if '1x1x1' in summary['dispatch_types']:
            reset_count = summary['dispatch_types']['1x1x1']
            print(f"  ℹ️  {reset_count} single-workgroup dispatches (likely reset passes)")

    print()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    html_path = sys.argv[1]

    if not Path(html_path).exists():
        print(f"Error: File not found: {html_path}")
        sys.exit(1)

    results = parse_webgpu_recording(html_path)
    print_report(results, html_path)


if __name__ == '__main__':
    main()
