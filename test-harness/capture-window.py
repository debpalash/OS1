#!/usr/bin/env python3
"""
IndOS QEMU Window Capture — X11 fallback for GTK display mode.

Finds the QEMU window by title, ensures it's visible and properly sized,
then captures a screenshot. Works even if QEMU is minimized.

Usage:
    python3 capture-window.py [--title "IndOS Test"] [--output screenshot.png]

Methods (in priority order):
    1. QEMU monitor screendump (framebuffer — works at ALL stages)
    2. xdotool + import (X11 window capture — GTK mode only)
    3. scrot (fullscreen fallback)
"""

import argparse
import subprocess
import socket
import sys
import time
import os

def qemu_screendump(monitor_sock: str, output: str) -> bool:
    """Capture via QEMU monitor — works at BIOS, boot, desktop. Best method."""
    try:
        ppm = output.replace('.png', '.ppm')
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.connect(monitor_sock)
        s.recv(4096)  # consume banner
        s.sendall(f'screendump {ppm}\n'.encode())
        time.sleep(0.5)
        s.recv(4096)
        s.close()
        
        if os.path.exists(ppm):
            subprocess.run(['convert', ppm, output], check=True, capture_output=True)
            os.remove(ppm)
            print(f"[screendump] Captured: {output}")
            return True
    except Exception as e:
        print(f"[screendump] Failed: {e}", file=sys.stderr)
    return False

def find_window(title: str) -> str | None:
    """Find X11 window ID by title substring."""
    try:
        result = subprocess.run(
            ['xdotool', 'search', '--name', title],
            capture_output=True, text=True, timeout=5
        )
        windows = result.stdout.strip().split('\n')
        return windows[0] if windows and windows[0] else None
    except Exception:
        return None

def ensure_window_visible(wid: str, min_w: int = 800, min_h: int = 600):
    """Activate, unminimize, and resize window if needed."""
    try:
        # Unminimize / activate
        subprocess.run(['xdotool', 'windowactivate', '--sync', wid], 
                      capture_output=True, timeout=5)
        time.sleep(0.3)
        
        # Get current geometry
        result = subprocess.run(
            ['xdotool', 'getwindowgeometry', '--shell', wid],
            capture_output=True, text=True, timeout=5
        )
        geom = {}
        for line in result.stdout.strip().split('\n'):
            if '=' in line:
                k, v = line.split('=', 1)
                geom[k] = int(v)
        
        w = geom.get('WIDTH', 0)
        h = geom.get('HEIGHT', 0)
        
        if w < min_w or h < min_h:
            new_w = max(w, min_w)
            new_h = max(h, min_h)
            subprocess.run(
                ['xdotool', 'windowsize', '--sync', wid, str(new_w), str(new_h)],
                capture_output=True, timeout=5
            )
            time.sleep(0.5)
            print(f"[window] Resized from {w}x{h} to {new_w}x{new_h}")
        
    except Exception as e:
        print(f"[window] Warning: {e}", file=sys.stderr)

def x11_capture(title: str, output: str) -> bool:
    """Capture via X11 window — works in GTK display mode."""
    wid = find_window(title)
    if not wid:
        print(f"[x11] Window '{title}' not found", file=sys.stderr)
        return False
    
    ensure_window_visible(wid)
    
    try:
        # Use ImageMagick's import to capture specific window
        subprocess.run(
            ['import', '-window', wid, output],
            capture_output=True, timeout=10, check=True
        )
        print(f"[x11] Captured window {wid}: {output}")
        return True
    except Exception as e:
        print(f"[x11] Capture failed: {e}", file=sys.stderr)
    return False

def scrot_capture(output: str) -> bool:
    """Fullscreen capture as last resort."""
    try:
        subprocess.run(['scrot', output], capture_output=True, timeout=10, check=True)
        print(f"[scrot] Fullscreen capture: {output}")
        return True
    except Exception:
        return False

def main():
    parser = argparse.ArgumentParser(description='Capture QEMU window screenshot')
    parser.add_argument('--title', default='IndOS Test', help='Window title to find')
    parser.add_argument('--output', '-o', default='screenshot.png', help='Output path')
    parser.add_argument('--monitor', '-m', default=None, help='QEMU monitor socket path')
    args = parser.parse_args()
    
    # Try methods in priority order
    
    # 1. QEMU monitor screendump (best — works everywhere)
    if args.monitor and os.path.exists(args.monitor):
        if qemu_screendump(args.monitor, args.output):
            return 0
    
    # Auto-find monitor socket
    for sock in [
        'test-harness/results/qemu-monitor.sock',
        'test-logs/qemu-monitor.sock',
    ]:
        path = os.path.join(os.environ.get('PROJECT_DIR', '.'), sock)
        if os.path.exists(path):
            if qemu_screendump(path, args.output):
                return 0
    
    # 2. X11 window capture (GTK mode)
    if os.environ.get('DISPLAY'):
        if x11_capture(args.title, args.output):
            return 0
    
    # 3. Scrot fullscreen (last resort)
    if scrot_capture(args.output):
        return 0
    
    print("ERROR: All capture methods failed", file=sys.stderr)
    return 1

if __name__ == '__main__':
    sys.exit(main())
