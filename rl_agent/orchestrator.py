#!/usr/bin/env python3
"""
Master Orchestrator for SentientOS Continuous Learning
Coordinates monitoring, retraining, deployment, and rollback
"""

import asyncio
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional
import logging
import signal
import subprocess

from trace_monitor import TraceMonitor
from continuous_learning import ContinuousLearner

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class LearningOrchestrator:
    """Orchestrates the entire continuous learning loop"""
    
    def __init__(self):
        self.monitor = TraceMonitor(
            logs_dir="logs",
            window_size=100,
            check_interval=30
        )
        
        self.learner = ContinuousLearner(
            logs_dir="logs",
            policies_dir="policies",
            checkpoints_dir="rl_checkpoints"
        )
        
        self.running = True
        self.tasks = []
        
    async def handle_degradation_events(self):
        """Process degradation events and trigger retraining"""
        while self.running:
            try:
                # Check for degradation events
                if self.monitor.degradation_events:
                    critical_events = [e for e in self.monitor.degradation_events 
                                     if e.severity == 'critical']
                    
                    if critical_events:
                        logger.warning(f"Processing {len(critical_events)} critical degradation events")
                        
                        # Trigger retraining
                        should_retrain = await self.monitor.trigger_retraining(critical_events)
                        
                        if should_retrain:
                            # Launch retraining
                            result = await self.learner.launch_retraining(
                                input_traces=str(self.monitor.logs_dir / "live_window.jsonl"),
                                trigger_reason=", ".join([e.event_type for e in critical_events])
                            )
                            
                            if result["status"] == "success":
                                logger.info(f"Retraining completed: v{result['version']}")
                                
                                # Deploy new policy
                                deploy_result = await self.learner.deploy_policy(
                                    result["policy_path"],
                                    result["version"]
                                )
                                
                                if deploy_result["status"] == "success":
                                    logger.info(f"Successfully deployed policy v{result['version']}")
                                    
                                    # Compare with previous version
                                    if self.learner.current_version > 1:
                                        comparison = await self.learner.compare_versions(
                                            self.learner.current_version - 1,
                                            self.learner.current_version
                                        )
                                        
                                        # Check performance
                                        if comparison["status"] == "success":
                                            comp_data = comparison["comparison"]
                                            new_win_rate = comp_data.get("policy2_win_rate", 0.5)
                                            
                                            if new_win_rate < 0.45:
                                                logger.warning(f"New policy underperforming: {new_win_rate:.2%} win rate")
                                                
                                                # Rollback
                                                rollback_result = await self.learner.rollback_policy(
                                                    self.learner.current_version - 1
                                                )
                                                
                                                if rollback_result["status"] == "success":
                                                    logger.info("Successfully rolled back to previous policy")
                                                else:
                                                    logger.error(f"Rollback failed: {rollback_result}")
                                            else:
                                                logger.info(f"New policy performing well: {new_win_rate:.2%} win rate")
                                else:
                                    logger.error(f"Policy deployment failed: {deploy_result}")
                            else:
                                logger.error(f"Retraining failed: {result}")
                        
                        # Clear processed events
                        self.monitor.degradation_events.clear()
                
                # Wait before next check
                await asyncio.sleep(60)
                
            except Exception as e:
                logger.error(f"Error in degradation handler: {e}")
                await asyncio.sleep(60)
    
    async def update_shell_policy(self):
        """Update the shell with new policy versions"""
        while self.running:
            try:
                # Check for active policy
                active_policy = self.learner.policies_dir / "active_policy.pkl"
                
                if active_policy.exists():
                    # Notify shell to reload policy
                    cmd = ["sentient", "rl", "reload-policy", str(active_policy)]
                    
                    try:
                        process = await asyncio.create_subprocess_exec(
                            *cmd,
                            stdout=asyncio.subprocess.PIPE,
                            stderr=asyncio.subprocess.PIPE
                        )
                        
                        stdout, stderr = await process.communicate()
                        
                        if process.returncode == 0:
                            logger.info("Shell policy updated successfully")
                        else:
                            logger.warning(f"Shell policy update failed: {stderr.decode()}")
                            
                    except FileNotFoundError:
                        # Shell not available, skip
                        pass
                
                # Check every 5 minutes
                await asyncio.sleep(300)
                
            except Exception as e:
                logger.error(f"Error updating shell policy: {e}")
                await asyncio.sleep(300)
    
    async def periodic_evaluation(self):
        """Periodically evaluate current policy performance"""
        while self.running:
            try:
                # Wait 30 minutes between evaluations
                await asyncio.sleep(1800)
                
                if self.learner.current_version > 0:
                    logger.info("Running periodic policy evaluation")
                    
                    # Evaluate current policy
                    current_policy = self.learner.policies_dir / f"rl_policy_v{self.learner.current_version}.pkl"
                    
                    if current_policy.exists():
                        cmd = [
                            "python3", "rl_agent/evaluate_agent.py",
                            "--policy", str(current_policy),
                            "--test-data", "rl_data/test.jsonl",
                            "--output-report", f"experiments/periodic_eval_v{self.learner.current_version}_{datetime.now():%Y%m%d_%H%M}.json"
                        ]
                        
                        process = await asyncio.create_subprocess_exec(
                            *cmd,
                            stdout=asyncio.subprocess.PIPE,
                            stderr=asyncio.subprocess.PIPE
                        )
                        
                        stdout, stderr = await process.communicate()
                        
                        if process.returncode == 0:
                            logger.info("Periodic evaluation completed")
                        else:
                            logger.error(f"Evaluation failed: {stderr.decode()}")
                
            except Exception as e:
                logger.error(f"Error in periodic evaluation: {e}")
    
    def handle_shutdown(self, signum, frame):
        """Graceful shutdown"""
        logger.info("Shutdown signal received")
        self.running = False
        
        # Cancel all tasks
        for task in self.tasks:
            task.cancel()
    
    async def run(self):
        """Run the orchestrator"""
        logger.info("Starting SentientOS Continuous Learning Orchestrator")
        
        # Set up signal handlers
        signal.signal(signal.SIGINT, self.handle_shutdown)
        signal.signal(signal.SIGTERM, self.handle_shutdown)
        
        # Create necessary directories
        for dir_path in ["logs", "policies", "rl_checkpoints", "experiments"]:
            Path(dir_path).mkdir(exist_ok=True)
        
        # Start all components
        self.tasks = [
            asyncio.create_task(self.monitor.monitor_loop()),
            asyncio.create_task(self.handle_degradation_events()),
            asyncio.create_task(self.update_shell_policy()),
            asyncio.create_task(self.periodic_evaluation())
        ]
        
        logger.info("All components started")
        logger.info("Monitoring traces for degradation...")
        logger.info("Press Ctrl+C to stop")
        
        # Wait for tasks
        try:
            await asyncio.gather(*self.tasks)
        except asyncio.CancelledError:
            logger.info("Tasks cancelled")
        
        logger.info("Orchestrator stopped")


async def main():
    """Main entry point"""
    orchestrator = LearningOrchestrator()
    await orchestrator.run()


if __name__ == "__main__":
    # Run with proper event loop
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Orchestrator terminated by user")
    except Exception as e:
        logger.error(f"Orchestrator error: {e}")
        sys.exit(1)