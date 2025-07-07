#!/usr/bin/env python3
"""
Guardrails and Halting Conditions for SentientOS

This module implements safety mechanisms to prevent runaway execution,
resource exhaustion, and harmful operations.
"""

import asyncio
import logging
import psutil
import time
from typing import Dict, Any, List, Optional, Callable, Set, Tuple
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime, timedelta
import re

logger = logging.getLogger(__name__)


class ViolationType(Enum):
    """Types of guardrail violations"""
    RESOURCE_LIMIT = "resource_limit"
    TIME_LIMIT = "time_limit"
    LOOP_DETECTED = "loop_detected"
    UNSAFE_OPERATION = "unsafe_operation"
    RATE_LIMIT = "rate_limit"
    PERMISSION_DENIED = "permission_denied"
    CONFIDENCE_THRESHOLD = "confidence_threshold"


@dataclass
class Violation:
    """Record of a guardrail violation"""
    violation_type: ViolationType
    severity: str  # "warning", "error", "critical"
    message: str
    timestamp: datetime = field(default_factory=datetime.now)
    context: Dict[str, Any] = field(default_factory=dict)
    
    def should_halt(self) -> bool:
        """Determine if this violation should halt execution"""
        return self.severity in ["error", "critical"]


@dataclass
class ResourceLimits:
    """Resource usage limits"""
    max_memory_mb: int = 4096
    max_cpu_percent: float = 80.0
    max_disk_io_mb: int = 1000
    max_network_connections: int = 100
    max_open_files: int = 1000
    max_execution_time_seconds: int = 300
    max_step_time_seconds: int = 30


@dataclass
class SafetyPolicy:
    """Safety policy configuration"""
    # Prohibited operations
    prohibited_commands: Set[str] = field(default_factory=lambda: {
        "rm -rf /", "format", "fdisk", "dd if=/dev/zero",
        "shutdown", "reboot", "kill -9 1"
    })
    
    # Sensitive paths
    protected_paths: Set[str] = field(default_factory=lambda: {
        "/etc", "/sys", "/proc", "/boot", "/dev",
        "C:\\Windows", "C:\\System32"
    })
    
    # Rate limits
    max_operations_per_minute: int = 100
    max_failures_per_window: int = 10
    failure_window_minutes: int = 5
    
    # Confidence thresholds
    min_tool_confidence: float = 0.3
    min_plan_confidence: float = 0.5
    
    # Loop detection
    max_repeated_steps: int = 3
    loop_detection_window: int = 10


class ResourceMonitor:
    """Monitor system resource usage"""
    
    def __init__(self, limits: ResourceLimits):
        self.limits = limits
        self.start_time = time.time()
        self.baseline_memory = psutil.Process().memory_info().rss / 1024 / 1024
    
    def check_resources(self) -> List[Violation]:
        """Check current resource usage against limits"""
        violations = []
        
        # Memory check
        current_memory = psutil.Process().memory_info().rss / 1024 / 1024
        memory_delta = current_memory - self.baseline_memory
        
        if memory_delta > self.limits.max_memory_mb:
            violations.append(Violation(
                violation_type=ViolationType.RESOURCE_LIMIT,
                severity="error",
                message=f"Memory usage exceeded: {memory_delta:.1f}MB > {self.limits.max_memory_mb}MB",
                context={"memory_mb": memory_delta}
            ))
        
        # CPU check
        cpu_percent = psutil.Process().cpu_percent(interval=0.1)
        if cpu_percent > self.limits.max_cpu_percent:
            violations.append(Violation(
                violation_type=ViolationType.RESOURCE_LIMIT,
                severity="warning" if cpu_percent < 90 else "error",
                message=f"CPU usage high: {cpu_percent:.1f}%",
                context={"cpu_percent": cpu_percent}
            ))
        
        # Time check
        elapsed = time.time() - self.start_time
        if elapsed > self.limits.max_execution_time_seconds:
            violations.append(Violation(
                violation_type=ViolationType.TIME_LIMIT,
                severity="critical",
                message=f"Execution time exceeded: {elapsed:.1f}s > {self.limits.max_execution_time_seconds}s",
                context={"elapsed_seconds": elapsed}
            ))
        
        # File descriptors
        open_files = len(psutil.Process().open_files())
        if open_files > self.limits.max_open_files:
            violations.append(Violation(
                violation_type=ViolationType.RESOURCE_LIMIT,
                severity="warning",
                message=f"Too many open files: {open_files}",
                context={"open_files": open_files}
            ))
        
        return violations


class OperationValidator:
    """Validate operations for safety"""
    
    def __init__(self, policy: SafetyPolicy):
        self.policy = policy
        self.operation_history: List[Dict[str, Any]] = []
        self.failure_times: List[datetime] = []
    
    def validate_operation(self, 
                          operation: str,
                          tool: str,
                          inputs: Dict[str, Any]) -> List[Violation]:
        """Validate an operation before execution"""
        violations = []
        
        # Check prohibited commands
        for prohibited in self.policy.prohibited_commands:
            if prohibited in operation.lower():
                violations.append(Violation(
                    violation_type=ViolationType.UNSAFE_OPERATION,
                    severity="critical",
                    message=f"Prohibited operation detected: {prohibited}",
                    context={"operation": operation, "tool": tool}
                ))
        
        # Check protected paths
        for key, value in inputs.items():
            if isinstance(value, str):
                for protected in self.policy.protected_paths:
                    if protected in value:
                        violations.append(Violation(
                            violation_type=ViolationType.PERMISSION_DENIED,
                            severity="error",
                            message=f"Access to protected path denied: {protected}",
                            context={"path": value, "input_key": key}
                        ))
        
        # Rate limiting
        self._add_operation(operation, tool)
        if self._check_rate_limit():
            violations.append(Violation(
                violation_type=ViolationType.RATE_LIMIT,
                severity="warning",
                message="Operation rate limit approaching",
                context={"operations_per_minute": len(self.operation_history)}
            ))
        
        return violations
    
    def validate_confidence(self, confidence: float, context: str) -> Optional[Violation]:
        """Validate confidence levels"""
        min_confidence = (self.policy.min_tool_confidence 
                         if context == "tool" 
                         else self.policy.min_plan_confidence)
        
        if confidence < min_confidence:
            return Violation(
                violation_type=ViolationType.CONFIDENCE_THRESHOLD,
                severity="warning",
                message=f"Low confidence for {context}: {confidence:.2f} < {min_confidence}",
                context={"confidence": confidence, "threshold": min_confidence}
            )
        return None
    
    def check_failure_rate(self) -> Optional[Violation]:
        """Check if failure rate is too high"""
        # Clean old failures
        cutoff = datetime.now() - timedelta(minutes=self.policy.failure_window_minutes)
        self.failure_times = [t for t in self.failure_times if t > cutoff]
        
        if len(self.failure_times) >= self.policy.max_failures_per_window:
            return Violation(
                violation_type=ViolationType.RATE_LIMIT,
                severity="error",
                message=f"High failure rate: {len(self.failure_times)} failures in {self.policy.failure_window_minutes} minutes",
                context={"failure_count": len(self.failure_times)}
            )
        return None
    
    def record_failure(self):
        """Record a failure for rate limiting"""
        self.failure_times.append(datetime.now())
    
    def _add_operation(self, operation: str, tool: str):
        """Add operation to history"""
        self.operation_history.append({
            "operation": operation,
            "tool": tool,
            "timestamp": datetime.now()
        })
        
        # Keep only recent operations
        cutoff = datetime.now() - timedelta(minutes=1)
        self.operation_history = [
            op for op in self.operation_history 
            if op["timestamp"] > cutoff
        ]
    
    def _check_rate_limit(self) -> bool:
        """Check if approaching rate limit"""
        return len(self.operation_history) > self.policy.max_operations_per_minute * 0.8


class LoopDetector:
    """Detect execution loops and repetitive patterns"""
    
    def __init__(self, policy: SafetyPolicy):
        self.policy = policy
        self.step_history: List[str] = []
        self.pattern_counts: Dict[str, int] = {}
    
    def check_for_loops(self, step_id: str) -> Optional[Violation]:
        """Check if we're in a loop"""
        self.step_history.append(step_id)
        
        # Keep only recent history
        if len(self.step_history) > self.policy.loop_detection_window * 2:
            self.step_history = self.step_history[-self.policy.loop_detection_window:]
        
        # Look for repeated patterns
        for length in range(2, min(len(self.step_history) // 2, 5) + 1):
            pattern = self._find_repeated_pattern(length)
            if pattern:
                pattern_str = "->".join(pattern)
                self.pattern_counts[pattern_str] = self.pattern_counts.get(pattern_str, 0) + 1
                
                if self.pattern_counts[pattern_str] >= self.policy.max_repeated_steps:
                    return Violation(
                        violation_type=ViolationType.LOOP_DETECTED,
                        severity="error",
                        message=f"Execution loop detected: {pattern_str}",
                        context={
                            "pattern": pattern,
                            "repetitions": self.pattern_counts[pattern_str]
                        }
                    )
        
        return None
    
    def _find_repeated_pattern(self, length: int) -> Optional[List[str]]:
        """Find repeated pattern of given length"""
        if len(self.step_history) < length * 2:
            return None
        
        recent = self.step_history[-length:]
        previous = self.step_history[-length*2:-length]
        
        if recent == previous:
            return recent
        
        return None


class GuardrailSystem:
    """Main guardrail system coordinating all safety checks"""
    
    def __init__(self,
                 resource_limits: Optional[ResourceLimits] = None,
                 safety_policy: Optional[SafetyPolicy] = None):
        self.resource_limits = resource_limits or ResourceLimits()
        self.safety_policy = safety_policy or SafetyPolicy()
        self.resource_monitor = ResourceMonitor(self.resource_limits)
        self.operation_validator = OperationValidator(self.safety_policy)
        self.loop_detector = LoopDetector(self.safety_policy)
        self.violations: List[Violation] = []
        self.halt_requested = False
        self.callbacks: List[Callable[[Violation], None]] = []
    
    def register_callback(self, callback: Callable[[Violation], None]):
        """Register callback for violations"""
        self.callbacks.append(callback)
    
    async def check_all(self, context: Dict[str, Any]) -> Tuple[bool, List[Violation]]:
        """Run all guardrail checks"""
        violations = []
        
        # Resource checks
        violations.extend(self.resource_monitor.check_resources())
        
        # Loop detection
        if "step_id" in context:
            loop_violation = self.loop_detector.check_for_loops(context["step_id"])
            if loop_violation:
                violations.append(loop_violation)
        
        # Confidence check
        if "confidence" in context:
            conf_violation = self.operation_validator.validate_confidence(
                context["confidence"],
                context.get("confidence_context", "unknown")
            )
            if conf_violation:
                violations.append(conf_violation)
        
        # Failure rate check
        failure_violation = self.operation_validator.check_failure_rate()
        if failure_violation:
            violations.append(failure_violation)
        
        # Process violations
        for violation in violations:
            logger.warning(f"Guardrail violation: {violation.message}")
            self.violations.append(violation)
            
            # Notify callbacks
            for callback in self.callbacks:
                try:
                    callback(violation)
                except Exception as e:
                    logger.error(f"Callback error: {e}")
            
            # Check if we should halt
            if violation.should_halt():
                self.halt_requested = True
        
        return not self.halt_requested, violations
    
    def validate_operation(self,
                          operation: str,
                          tool: str,
                          inputs: Dict[str, Any]) -> Tuple[bool, List[Violation]]:
        """Validate a specific operation"""
        violations = self.operation_validator.validate_operation(operation, tool, inputs)
        
        for violation in violations:
            self.violations.append(violation)
            if violation.should_halt():
                self.halt_requested = True
        
        return len(violations) == 0, violations
    
    def record_failure(self):
        """Record operation failure"""
        self.operation_validator.record_failure()
    
    def get_status(self) -> Dict[str, Any]:
        """Get current guardrail status"""
        return {
            "halt_requested": self.halt_requested,
            "total_violations": len(self.violations),
            "recent_violations": [
                {
                    "type": v.violation_type.value,
                    "severity": v.severity,
                    "message": v.message,
                    "timestamp": v.timestamp.isoformat()
                }
                for v in self.violations[-10:]  # Last 10
            ],
            "resource_usage": {
                "memory_mb": psutil.Process().memory_info().rss / 1024 / 1024,
                "cpu_percent": psutil.Process().cpu_percent(interval=0.1),
                "open_files": len(psutil.Process().open_files())
            }
        }
    
    def reset(self):
        """Reset guardrail state"""
        self.violations.clear()
        self.halt_requested = False
        self.resource_monitor = ResourceMonitor(self.resource_limits)


async def demo_guardrails():
    """Demonstrate guardrail system"""
    # Create guardrail system
    guardrails = GuardrailSystem()
    
    # Register violation callback
    def on_violation(violation: Violation):
        print(f"⚠️  Violation: [{violation.severity}] {violation.message}")
    
    guardrails.register_callback(on_violation)
    
    print("🛡️ Guardrail System Demo\n")
    
    # Test 1: Safe operation
    print("Test 1: Safe operation")
    safe, violations = guardrails.validate_operation(
        "check memory usage",
        "memory_check",
        {}
    )
    print(f"  Result: {'✅ Safe' if safe else '❌ Unsafe'}")
    
    # Test 2: Unsafe operation
    print("\nTest 2: Unsafe operation")
    safe, violations = guardrails.validate_operation(
        "rm -rf /etc/config",
        "shell_execute",
        {"path": "/etc/config"}
    )
    print(f"  Result: {'✅ Safe' if safe else '❌ Unsafe'}")
    
    # Test 3: Loop detection
    print("\nTest 3: Loop detection")
    for i in range(10):
        context = {"step_id": f"step_{i % 3}"}  # Creates loop pattern
        safe, violations = await guardrails.check_all(context)
        if not safe:
            print(f"  Loop detected at iteration {i}")
            break
    
    # Test 4: Resource monitoring
    print("\nTest 4: Resource monitoring")
    status = guardrails.get_status()
    print(f"  Memory: {status['resource_usage']['memory_mb']:.1f}MB")
    print(f"  CPU: {status['resource_usage']['cpu_percent']:.1f}%")
    print(f"  Total violations: {status['total_violations']}")


if __name__ == "__main__":
    asyncio.run(demo_guardrails())