#!/usr/bin/env python3
"""
Terminal UI Dashboard for SentientOS Continuous Learning
Real-time monitoring of system performance and learning metrics
"""

import asyncio
import json
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional
from collections import deque, defaultdict
import curses
from dataclasses import dataclass
import numpy as np


@dataclass
class SystemMetrics:
    """Current system metrics"""
    avg_reward: float
    avg_confidence: float
    fallback_rate: float
    tool_accuracy: float
    policy_version: int
    total_traces: int
    traces_per_minute: float
    last_update: datetime


class Dashboard:
    """TUI Dashboard for monitoring SentientOS"""
    
    def __init__(self, logs_dir: str = "logs", policies_dir: str = "policies"):
        self.logs_dir = Path(logs_dir)
        self.policies_dir = Path(policies_dir)
        
        # Metrics tracking
        self.reward_history = deque(maxlen=100)
        self.confidence_history = deque(maxlen=100)
        self.intent_failures = defaultdict(int)
        self.intent_totals = defaultdict(int)
        
        # Current state
        self.current_metrics = SystemMetrics(
            avg_reward=0.0,
            avg_confidence=0.0,
            fallback_rate=0.0,
            tool_accuracy=0.0,
            policy_version=1,
            total_traces=0,
            traces_per_minute=0.0,
            last_update=datetime.now()
        )
        
        self.degradation_events = deque(maxlen=10)
        self.recent_traces = deque(maxlen=20)
        
    def create_histogram(self, data: List[float], width: int = 40, height: int = 10) -> List[str]:
        """Create ASCII histogram"""
        if not data:
            return ["No data"]
        
        # Create bins
        min_val, max_val = min(data), max(data)
        if min_val == max_val:
            bins = [min_val]
        else:
            bins = np.linspace(min_val, max_val, min(10, len(set(data))))
        
        hist, _ = np.histogram(data, bins=bins)
        
        # Normalize to height
        if hist.max() > 0:
            hist_norm = (hist * height / hist.max()).astype(int)
        else:
            hist_norm = hist
        
        # Create visual
        lines = []
        for i in range(height, 0, -1):
            line = ""
            for h in hist_norm:
                if h >= i:
                    line += "█"
                else:
                    line += " "
            lines.append(line)
        
        # Add axis
        lines.append("-" * len(hist_norm))
        lines.append(f"{min_val:.2f} " + " " * (len(hist_norm) - 10) + f" {max_val:.2f}")
        
        return lines
    
    def format_time_ago(self, timestamp: datetime) -> str:
        """Format timestamp as time ago"""
        delta = datetime.now() - timestamp
        if delta.seconds < 60:
            return f"{delta.seconds}s ago"
        elif delta.seconds < 3600:
            return f"{delta.seconds // 60}m ago"
        else:
            return f"{delta.seconds // 3600}h ago"
    
    async def update_metrics(self):
        """Update metrics from log files"""
        # Read recent traces
        trace_files = sorted(self.logs_dir.glob("*.jsonl"), key=os.path.getmtime, reverse=True)
        
        traces = []
        for trace_file in trace_files[:5]:  # Last 5 files
            with open(trace_file, 'r') as f:
                for line in f:
                    try:
                        trace = json.loads(line.strip())
                        if 'reward' in trace:
                            traces.append(trace)
                    except:
                        continue
        
        if traces:
            # Update histories
            for trace in traces[-100:]:
                self.reward_history.append(trace.get('reward', 0))
                if 'confidence' in trace:
                    self.confidence_history.append(trace['confidence'])
                
                # Track intent performance
                intent = trace.get('intent', 'unknown')
                self.intent_totals[intent] += 1
                if trace.get('reward', 0) == 0:
                    self.intent_failures[intent] += 1
            
            # Calculate current metrics
            recent_traces = traces[-20:]
            self.current_metrics.avg_reward = np.mean([t.get('reward', 0) for t in recent_traces])
            
            confidences = [t['confidence'] for t in recent_traces if 'confidence' in t]
            self.current_metrics.avg_confidence = np.mean(confidences) if confidences else 0
            
            fallbacks = sum(1 for t in recent_traces if t.get('fallback_used', False))
            self.current_metrics.fallback_rate = fallbacks / len(recent_traces)
            
            self.current_metrics.total_traces = len(traces)
            self.recent_traces = deque(recent_traces, maxlen=20)
        
        # Get current policy version
        policy_files = list(self.policies_dir.glob("rl_policy_v*.pkl"))
        if policy_files:
            versions = []
            for f in policy_files:
                try:
                    version = int(f.stem.split('_v')[1])
                    versions.append(version)
                except:
                    continue
            self.current_metrics.policy_version = max(versions) if versions else 1
        
        # Read degradation events
        event_files = sorted(self.logs_dir.glob("degradation_events_*.jsonl"), reverse=True)
        if event_files:
            with open(event_files[0], 'r') as f:
                events = []
                for line in f:
                    try:
                        event = json.loads(line.strip())
                        events.append(event)
                    except:
                        continue
                self.degradation_events = deque(events[-10:], maxlen=10)
        
        self.current_metrics.last_update = datetime.now()
    
    def draw_dashboard(self, stdscr):
        """Draw the dashboard UI"""
        curses.curs_set(0)  # Hide cursor
        stdscr.nodelay(1)   # Non-blocking input
        
        # Colors
        curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)
        curses.init_pair(2, curses.COLOR_RED, curses.COLOR_BLACK)
        curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)
        curses.init_pair(4, curses.COLOR_CYAN, curses.COLOR_BLACK)
        
        while True:
            height, width = stdscr.getmaxyx()
            stdscr.clear()
            
            # Header
            header = "🚀 SentientOS Continuous Learning Dashboard"
            stdscr.addstr(0, (width - len(header)) // 2, header, curses.A_BOLD)
            stdscr.addstr(1, 0, "=" * width)
            
            # System Status
            row = 3
            stdscr.addstr(row, 2, "📊 System Status", curses.A_BOLD)
            row += 1
            
            status_color = curses.color_pair(1) if self.current_metrics.avg_reward > 0.7 else curses.color_pair(2)
            stdscr.addstr(row, 4, f"Policy Version: v{self.current_metrics.policy_version}", curses.color_pair(4))
            row += 1
            stdscr.addstr(row, 4, f"Avg Reward: {self.current_metrics.avg_reward:.2f}", status_color)
            row += 1
            stdscr.addstr(row, 4, f"Avg Confidence: {self.current_metrics.avg_confidence:.2f}")
            row += 1
            stdscr.addstr(row, 4, f"Fallback Rate: {self.current_metrics.fallback_rate:.1%}")
            row += 1
            stdscr.addstr(row, 4, f"Total Traces: {self.current_metrics.total_traces}")
            row += 2
            
            # Reward Histogram
            if self.reward_history:
                stdscr.addstr(row, 2, "📈 Reward Distribution", curses.A_BOLD)
                row += 1
                
                hist_lines = self.create_histogram(list(self.reward_history), width=40, height=6)
                for line in hist_lines:
                    stdscr.addstr(row, 4, line)
                    row += 1
                row += 1
            
            # Top Failing Intents
            if self.intent_failures:
                stdscr.addstr(row, 2, "⚠️  Top Failing Intents", curses.A_BOLD)
                row += 1
                
                # Calculate failure rates
                failure_rates = []
                for intent, failures in self.intent_failures.items():
                    total = self.intent_totals[intent]
                    if total > 0:
                        rate = failures / total
                        failure_rates.append((intent, rate, failures, total))
                
                # Sort by failure rate
                failure_rates.sort(key=lambda x: x[1], reverse=True)
                
                for i, (intent, rate, failures, total) in enumerate(failure_rates[:5]):
                    color = curses.color_pair(2) if rate > 0.3 else curses.color_pair(3)
                    stdscr.addstr(row, 4, f"{intent:<20} {rate:>6.1%} ({failures}/{total})", color)
                    row += 1
                row += 1
            
            # Recent Degradation Events
            if self.degradation_events:
                stdscr.addstr(row, 2, "🔔 Recent Degradation Events", curses.A_BOLD)
                row += 1
                
                for event in list(self.degradation_events)[-3:]:
                    timestamp = datetime.fromisoformat(event['timestamp'])
                    time_ago = self.format_time_ago(timestamp)
                    severity_color = curses.color_pair(2) if event['severity'] == 'critical' else curses.color_pair(3)
                    
                    stdscr.addstr(row, 4, f"[{time_ago}] {event['event_type']}", severity_color)
                    row += 1
                row += 1
            
            # Last Update
            stdscr.addstr(height - 2, 2, 
                         f"Last update: {self.current_metrics.last_update:%H:%M:%S} | Press 'q' to quit",
                         curses.A_DIM)
            
            stdscr.refresh()
            
            # Check for quit
            key = stdscr.getch()
            if key == ord('q'):
                break
            
            # Update every 5 seconds
            curses.napms(5000)
            
            # Update metrics
            asyncio.run(self.update_metrics())
    
    def run(self):
        """Run the dashboard"""
        # Initial update
        asyncio.run(self.update_metrics())
        
        # Run curses UI
        curses.wrapper(self.draw_dashboard)


def main():
    """Main entry point"""
    import os
    
    dashboard = Dashboard()
    dashboard.run()


if __name__ == "__main__":
    main()