#!/usr/bin/env python3
"""
Live Trace Monitoring System for SentientOS
Continuously monitors execution traces and triggers retraining when needed
"""

import json
import time
import os
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Deque
from collections import deque
from dataclasses import dataclass, asdict
import logging
import asyncio
import aiofiles
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class TraceMetrics:
    """Aggregated metrics from recent traces"""
    avg_reward: float
    avg_confidence: float
    fallback_rate: float
    consecutive_failures: int
    tool_mismatch_rate: float
    total_traces: int
    window_start: datetime
    window_end: datetime


@dataclass
class DegradationEvent:
    """Represents a detected degradation event"""
    event_type: str  # 'low_reward', 'high_fallback', 'low_confidence', 'consecutive_failures'
    timestamp: datetime
    metrics: TraceMetrics
    severity: str  # 'warning', 'critical'
    message: str


class TraceMonitor:
    """Monitors live traces and detects degradation"""
    
    def __init__(self, 
                 logs_dir: str = "logs",
                 window_size: int = 100,
                 check_interval: int = 30):
        self.logs_dir = Path(logs_dir)
        self.window_size = window_size
        self.check_interval = check_interval
        
        # Rolling window of recent traces
        self.trace_window: Deque[Dict] = deque(maxlen=window_size)
        
        # Degradation thresholds
        self.thresholds = {
            'min_avg_reward': 0.5,
            'max_fallback_rate': 0.2,
            'min_avg_confidence': 0.4,
            'max_consecutive_failures': 10
        }
        
        # State tracking
        self.last_check = datetime.now()
        self.policy_version = "1.0"
        self.degradation_events: List[DegradationEvent] = []
        
    def parse_trace_line(self, line: str) -> Optional[Dict]:
        """Parse a single trace line from JSONL"""
        try:
            trace = json.loads(line.strip())
            # Validate required fields
            required = ['timestamp', 'reward', 'intent']
            if all(field in trace for field in required):
                return trace
        except json.JSONDecodeError:
            logger.warning(f"Failed to parse trace line: {line[:50]}...")
        return None
    
    def calculate_metrics(self) -> TraceMetrics:
        """Calculate aggregated metrics from trace window"""
        if not self.trace_window:
            return TraceMetrics(
                avg_reward=0.0,
                avg_confidence=0.0,
                fallback_rate=0.0,
                consecutive_failures=0,
                tool_mismatch_rate=0.0,
                total_traces=0,
                window_start=datetime.now(),
                window_end=datetime.now()
            )
        
        # Calculate averages
        rewards = [t.get('reward', 0) for t in self.trace_window]
        confidences = [t.get('confidence', 0) for t in self.trace_window if 'confidence' in t]
        
        avg_reward = sum(rewards) / len(rewards) if rewards else 0
        avg_confidence = sum(confidences) / len(confidences) if confidences else 0
        
        # Calculate rates
        fallback_count = sum(1 for t in self.trace_window if t.get('fallback_used', False))
        fallback_rate = fallback_count / len(self.trace_window)
        
        # Tool mismatch: when a tool was expected but not used correctly
        mismatch_count = sum(1 for t in self.trace_window 
                           if t.get('tool_expected') and t.get('tool_used') != t.get('tool_expected'))
        tool_mismatch_rate = mismatch_count / len(self.trace_window)
        
        # Consecutive failures
        consecutive_failures = 0
        for trace in reversed(self.trace_window):
            if trace.get('reward', 0) == 0:
                consecutive_failures += 1
            else:
                break
        
        # Time window
        timestamps = [datetime.fromisoformat(t['timestamp']) for t in self.trace_window 
                     if 'timestamp' in t]
        window_start = min(timestamps) if timestamps else datetime.now()
        window_end = max(timestamps) if timestamps else datetime.now()
        
        return TraceMetrics(
            avg_reward=avg_reward,
            avg_confidence=avg_confidence,
            fallback_rate=fallback_rate,
            consecutive_failures=consecutive_failures,
            tool_mismatch_rate=tool_mismatch_rate,
            total_traces=len(self.trace_window),
            window_start=window_start,
            window_end=window_end
        )
    
    def detect_degradation(self, metrics: TraceMetrics) -> List[DegradationEvent]:
        """Detect degradation events based on metrics"""
        events = []
        
        # Low average reward
        if metrics.avg_reward < self.thresholds['min_avg_reward']:
            events.append(DegradationEvent(
                event_type='low_reward',
                timestamp=datetime.now(),
                metrics=metrics,
                severity='critical' if metrics.avg_reward < 0.3 else 'warning',
                message=f"Average reward {metrics.avg_reward:.2f} below threshold {self.thresholds['min_avg_reward']}"
            ))
        
        # High fallback rate
        if metrics.fallback_rate > self.thresholds['max_fallback_rate']:
            events.append(DegradationEvent(
                event_type='high_fallback',
                timestamp=datetime.now(),
                metrics=metrics,
                severity='warning',
                message=f"Fallback rate {metrics.fallback_rate:.2%} exceeds threshold {self.thresholds['max_fallback_rate']:.2%}"
            ))
        
        # Low confidence
        if metrics.avg_confidence < self.thresholds['min_avg_confidence'] and metrics.avg_confidence > 0:
            events.append(DegradationEvent(
                event_type='low_confidence',
                timestamp=datetime.now(),
                metrics=metrics,
                severity='warning',
                message=f"Average confidence {metrics.avg_confidence:.2f} below threshold {self.thresholds['min_avg_confidence']}"
            ))
        
        # Consecutive failures
        if metrics.consecutive_failures > self.thresholds['max_consecutive_failures']:
            events.append(DegradationEvent(
                event_type='consecutive_failures',
                timestamp=datetime.now(),
                metrics=metrics,
                severity='critical',
                message=f"{metrics.consecutive_failures} consecutive failures exceed threshold {self.thresholds['max_consecutive_failures']}"
            ))
        
        return events
    
    async def load_recent_traces(self):
        """Load recent traces from log files"""
        trace_files = sorted(self.logs_dir.glob("*.jsonl"), key=os.path.getmtime, reverse=True)
        
        loaded_count = 0
        for trace_file in trace_files:
            if loaded_count >= self.window_size:
                break
                
            async with aiofiles.open(trace_file, 'r') as f:
                async for line in f:
                    trace = self.parse_trace_line(line)
                    if trace:
                        self.trace_window.append(trace)
                        loaded_count += 1
                        if loaded_count >= self.window_size:
                            break
        
        logger.info(f"Loaded {loaded_count} recent traces")
    
    async def save_degradation_event(self, event: DegradationEvent):
        """Save degradation event to log"""
        event_file = self.logs_dir / f"degradation_events_{datetime.now():%Y%m%d}.jsonl"
        
        async with aiofiles.open(event_file, 'a') as f:
            await f.write(json.dumps(asdict(event), default=str) + '\n')
    
    async def trigger_retraining(self, events: List[DegradationEvent]):
        """Trigger retraining based on degradation events"""
        logger.warning(f"RETRAIN_TRIGGER: {len(events)} degradation events detected")
        
        # Save current window as training data
        window_file = self.logs_dir / "live_window.jsonl"
        async with aiofiles.open(window_file, 'w') as f:
            for trace in self.trace_window:
                await f.write(json.dumps(trace) + '\n')
        
        # Log trigger event
        trigger_event = {
            'event': 'RETRAIN_TRIGGER',
            'timestamp': datetime.now().isoformat(),
            'degradation_events': [e.event_type for e in events],
            'metrics': asdict(events[0].metrics) if events else {},
            'window_size': len(self.trace_window)
        }
        
        trigger_file = self.logs_dir / f"retrain_triggers_{datetime.now():%Y%m%d}.jsonl"
        async with aiofiles.open(trigger_file, 'a') as f:
            await f.write(json.dumps(trigger_event) + '\n')
        
        logger.info(f"Saved {len(self.trace_window)} traces to {window_file}")
        
        # Return True to indicate retraining should proceed
        return True
    
    async def monitor_loop(self):
        """Main monitoring loop"""
        logger.info("Starting trace monitoring...")
        
        # Initial load
        await self.load_recent_traces()
        
        while True:
            try:
                # Calculate metrics
                metrics = self.calculate_metrics()
                
                # Detect degradation
                events = self.detect_degradation(metrics)
                
                if events:
                    logger.warning(f"Detected {len(events)} degradation events")
                    for event in events:
                        await self.save_degradation_event(event)
                        self.degradation_events.append(event)
                    
                    # Check if we should trigger retraining
                    critical_events = [e for e in events if e.severity == 'critical']
                    if critical_events:
                        await self.trigger_retraining(critical_events)
                
                # Log periodic status
                logger.info(f"Monitor status - Traces: {metrics.total_traces}, "
                          f"Avg Reward: {metrics.avg_reward:.2f}, "
                          f"Consecutive Failures: {metrics.consecutive_failures}")
                
                # Wait before next check
                await asyncio.sleep(self.check_interval)
                
            except Exception as e:
                logger.error(f"Monitor error: {e}")
                await asyncio.sleep(self.check_interval)


class TraceFileHandler(FileSystemEventHandler):
    """Handles new trace file events"""
    
    def __init__(self, monitor: TraceMonitor):
        self.monitor = monitor
        
    def on_modified(self, event):
        if event.src_path.endswith('.jsonl') and not event.is_directory:
            # Queue file for processing
            asyncio.create_task(self.process_new_traces(event.src_path))
    
    async def process_new_traces(self, filepath):
        """Process new traces from modified file"""
        try:
            async with aiofiles.open(filepath, 'r') as f:
                # Seek to near end to get latest traces
                await f.seek(0, 2)  # End of file
                pos = max(0, (await f.tell()) - 4096)  # Last 4KB
                await f.seek(pos)
                
                async for line in f:
                    trace = self.monitor.parse_trace_line(line)
                    if trace:
                        self.monitor.trace_window.append(trace)
        except Exception as e:
            logger.error(f"Error processing new traces: {e}")


async def main():
    """Main entry point"""
    monitor = TraceMonitor(
        logs_dir="logs",
        window_size=100,
        check_interval=30  # Check every 30 seconds
    )
    
    # Set up file system monitoring
    event_handler = TraceFileHandler(monitor)
    observer = Observer()
    observer.schedule(event_handler, str(monitor.logs_dir), recursive=False)
    observer.start()
    
    try:
        # Run monitoring loop
        await monitor.monitor_loop()
    finally:
        observer.stop()
        observer.join()


if __name__ == "__main__":
    asyncio.run(main())