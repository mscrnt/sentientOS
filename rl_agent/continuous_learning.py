#!/usr/bin/env python3
"""
Continuous Learning Pipeline for SentientOS
Automatically retrains policies based on degradation events
"""

import os
import json
import subprocess
import asyncio
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import logging
import shutil
import pickle

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class ContinuousLearner:
    """Manages the continuous learning pipeline"""
    
    def __init__(self, 
                 logs_dir: str = "logs",
                 policies_dir: str = "policies",
                 checkpoints_dir: str = "rl_checkpoints"):
        self.logs_dir = Path(logs_dir)
        self.policies_dir = Path(policies_dir)
        self.checkpoints_dir = Path(checkpoints_dir)
        
        # Create directories
        self.policies_dir.mkdir(exist_ok=True)
        self.checkpoints_dir.mkdir(exist_ok=True)
        
        # Track policy versions
        self.current_version = self._get_latest_version()
        self.training_in_progress = False
        
    def _get_latest_version(self) -> int:
        """Get the latest policy version number"""
        policy_files = list(self.policies_dir.glob("rl_policy_v*.pkl"))
        if not policy_files:
            return 1
        
        versions = []
        for f in policy_files:
            try:
                version = int(f.stem.split('_v')[1])
                versions.append(version)
            except:
                continue
        
        return max(versions) if versions else 1
    
    async def launch_retraining(self, 
                              input_traces: str,
                              trigger_reason: str) -> Dict[str, any]:
        """Launch PPO retraining with new data"""
        if self.training_in_progress:
            logger.warning("Training already in progress, skipping")
            return {"status": "skipped", "reason": "training_in_progress"}
        
        self.training_in_progress = True
        next_version = self.current_version + 1
        
        logger.info(f"Starting retraining for policy v{next_version}")
        logger.info(f"Trigger reason: {trigger_reason}")
        
        # Prepare training command
        output_policy = self.policies_dir / f"rl_policy_v{next_version}.pkl"
        checkpoint_dir = self.checkpoints_dir / f"v{next_version}"
        
        cmd = [
            "python3", "rl_agent/train_agent.py",
            "--train-data", input_traces,
            "--test-data", "rl_data/test.jsonl",
            "--output-policy", str(output_policy),
            "--checkpoint-dir", str(checkpoint_dir),
            "--epochs", "20",  # Fewer epochs for continuous learning
            "--batch-size", "32",
            "--learning-rate", "0.0001",  # Lower LR for fine-tuning
            "--early-stopping",
            "--patience", "3"
        ]
        
        # Log training start
        training_log = {
            "event": "TRAINING_START",
            "timestamp": datetime.now().isoformat(),
            "version": next_version,
            "trigger_reason": trigger_reason,
            "input_traces": input_traces,
            "command": " ".join(cmd)
        }
        
        await self._log_event(training_log)
        
        try:
            # Run training
            process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            stdout, stderr = await process.communicate()
            
            if process.returncode == 0:
                logger.info(f"Training completed successfully for v{next_version}")
                
                # Parse training results
                results = self._parse_training_output(stdout.decode())
                
                # Log success
                training_log = {
                    "event": "TRAINING_SUCCESS",
                    "timestamp": datetime.now().isoformat(),
                    "version": next_version,
                    "results": results
                }
                await self._log_event(training_log)
                
                return {
                    "status": "success",
                    "version": next_version,
                    "policy_path": str(output_policy),
                    "results": results
                }
            else:
                logger.error(f"Training failed: {stderr.decode()}")
                
                # Log failure
                training_log = {
                    "event": "TRAINING_FAILURE",
                    "timestamp": datetime.now().isoformat(),
                    "version": next_version,
                    "error": stderr.decode()
                }
                await self._log_event(training_log)
                
                return {
                    "status": "failed",
                    "version": next_version,
                    "error": stderr.decode()
                }
                
        except Exception as e:
            logger.error(f"Training exception: {e}")
            return {
                "status": "error",
                "version": next_version,
                "error": str(e)
            }
        finally:
            self.training_in_progress = False
    
    def _parse_training_output(self, output: str) -> Dict:
        """Parse training output for metrics"""
        results = {
            "final_reward": 0.0,
            "final_loss": 0.0,
            "best_epoch": 0,
            "total_epochs": 0
        }
        
        # Simple parsing - can be enhanced based on actual output format
        lines = output.split('\n')
        for line in lines:
            if "Final average reward:" in line:
                try:
                    results["final_reward"] = float(line.split(":")[-1].strip())
                except:
                    pass
            elif "Best epoch:" in line:
                try:
                    results["best_epoch"] = int(line.split(":")[-1].strip())
                except:
                    pass
        
        return results
    
    async def deploy_policy(self, 
                          policy_path: str,
                          version: int) -> Dict[str, any]:
        """Deploy new policy with safety checks"""
        logger.info(f"Deploying policy v{version}")
        
        # First, validate the policy
        validation = await self._validate_policy(policy_path)
        if not validation["valid"]:
            logger.error(f"Policy validation failed: {validation['reason']}")
            return {
                "status": "failed",
                "reason": f"validation_failed: {validation['reason']}"
            }
        
        # Run safety tests
        safety_tests = await self._run_safety_tests(policy_path)
        if not safety_tests["passed"]:
            logger.error(f"Safety tests failed: {safety_tests['failures']}")
            return {
                "status": "failed",
                "reason": f"safety_tests_failed: {safety_tests['failures']}"
            }
        
        # Deploy the policy
        try:
            # Copy to active policy location
            active_policy = self.policies_dir / "active_policy.pkl"
            shutil.copy2(policy_path, active_policy)
            
            # Update version tracking
            self.current_version = version
            
            # Log deployment
            deploy_log = {
                "event": "POLICY_DEPLOYED",
                "timestamp": datetime.now().isoformat(),
                "version": version,
                "validation": validation,
                "safety_tests": safety_tests
            }
            await self._log_event(deploy_log)
            
            return {
                "status": "success",
                "version": version,
                "deployed_at": datetime.now().isoformat()
            }
            
        except Exception as e:
            logger.error(f"Deployment error: {e}")
            return {
                "status": "error",
                "reason": str(e)
            }
    
    async def _validate_policy(self, policy_path: str) -> Dict[str, any]:
        """Validate policy file can be loaded and used"""
        try:
            # Load policy
            with open(policy_path, 'rb') as f:
                policy = pickle.load(f)
            
            # Basic validation - check it has expected methods/attributes
            if not hasattr(policy, 'select_action'):
                return {"valid": False, "reason": "Missing select_action method"}
            
            # Test inference
            test_state = {
                "intent_type": "PureQuery",
                "prompt_length": 50,
                "has_tool_keywords": False,
                "model_availability": {"gpt-4": True, "llama3": True}
            }
            
            action = policy.select_action(test_state)
            if action is None:
                return {"valid": False, "reason": "Inference returned None"}
            
            return {"valid": True, "reason": "All checks passed"}
            
        except Exception as e:
            return {"valid": False, "reason": f"Exception: {str(e)}"}
    
    async def _run_safety_tests(self, policy_path: str) -> Dict[str, any]:
        """Run safety tests with known-good inputs"""
        test_cases = [
            {
                "name": "simple_query",
                "state": {
                    "intent_type": "PureQuery",
                    "prompt_length": 20,
                    "has_tool_keywords": False,
                    "model_availability": {"gpt-4": True}
                },
                "expected_confidence_min": 0.3
            },
            {
                "name": "tool_action",
                "state": {
                    "intent_type": "PureAction",
                    "prompt_length": 50,
                    "has_tool_keywords": True,
                    "model_availability": {"gpt-4": True}
                },
                "expected_confidence_min": 0.3
            }
        ]
        
        try:
            with open(policy_path, 'rb') as f:
                policy = pickle.load(f)
            
            failures = []
            for test in test_cases:
                try:
                    action = policy.select_action(test["state"])
                    confidence = action.get("confidence", 0)
                    
                    if confidence < test["expected_confidence_min"]:
                        failures.append(f"{test['name']}: Low confidence {confidence}")
                        
                except Exception as e:
                    failures.append(f"{test['name']}: {str(e)}")
            
            return {
                "passed": len(failures) == 0,
                "failures": failures,
                "total_tests": len(test_cases)
            }
            
        except Exception as e:
            return {
                "passed": False,
                "failures": [f"Failed to load policy: {str(e)}"],
                "total_tests": 0
            }
    
    async def compare_versions(self, 
                             old_version: int,
                             new_version: int) -> Dict[str, any]:
        """Compare two policy versions"""
        logger.info(f"Comparing policy v{old_version} vs v{new_version}")
        
        old_policy = self.policies_dir / f"rl_policy_v{old_version}.pkl"
        new_policy = self.policies_dir / f"rl_policy_v{new_version}.pkl"
        
        cmd = [
            "python3", "rl_agent/evaluate_agent.py",
            "--policy1", str(old_policy),
            "--policy2", str(new_policy),
            "--test-data", "rl_data/test.jsonl",
            "--output-report", f"experiments/comparison_v{old_version}_vs_v{new_version}.json"
        ]
        
        try:
            process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            stdout, stderr = await process.communicate()
            
            if process.returncode == 0:
                # Parse comparison results
                report_path = f"experiments/comparison_v{old_version}_vs_v{new_version}.json"
                with open(report_path, 'r') as f:
                    comparison = json.load(f)
                
                return {
                    "status": "success",
                    "comparison": comparison
                }
            else:
                return {
                    "status": "failed",
                    "error": stderr.decode()
                }
                
        except Exception as e:
            return {
                "status": "error",
                "error": str(e)
            }
    
    async def rollback_policy(self, target_version: int) -> Dict[str, any]:
        """Rollback to a previous policy version"""
        logger.warning(f"ROLLBACK_TRIGGER: Rolling back to policy v{target_version}")
        
        target_policy = self.policies_dir / f"rl_policy_v{target_version}.pkl"
        
        if not target_policy.exists():
            return {
                "status": "failed",
                "reason": f"Policy v{target_version} not found"
            }
        
        # Deploy the older version
        result = await self.deploy_policy(str(target_policy), target_version)
        
        if result["status"] == "success":
            # Log rollback
            rollback_log = {
                "event": "POLICY_ROLLBACK",
                "timestamp": datetime.now().isoformat(),
                "from_version": self.current_version,
                "to_version": target_version,
                "reason": "Performance degradation detected"
            }
            await self._log_event(rollback_log)
        
        return result
    
    async def _log_event(self, event: Dict):
        """Log event to continuous learning log"""
        log_file = self.logs_dir / f"continuous_learning_{datetime.now():%Y%m%d}.jsonl"
        
        with open(log_file, 'a') as f:
            f.write(json.dumps(event) + '\n')
    
    async def monitor_and_retrain_loop(self):
        """Main loop that monitors for retrain triggers"""
        logger.info("Starting continuous learning monitor...")
        
        trigger_file = self.logs_dir / f"retrain_triggers_{datetime.now():%Y%m%d}.jsonl"
        
        last_position = 0
        while True:
            try:
                if trigger_file.exists():
                    with open(trigger_file, 'r') as f:
                        f.seek(last_position)
                        for line in f:
                            trigger = json.loads(line.strip())
                            
                            if trigger.get("event") == "RETRAIN_TRIGGER":
                                logger.info("Detected retrain trigger")
                                
                                # Launch retraining
                                result = await self.launch_retraining(
                                    input_traces=str(self.logs_dir / "live_window.jsonl"),
                                    trigger_reason=", ".join(trigger.get("degradation_events", []))
                                )
                                
                                if result["status"] == "success":
                                    # Deploy new policy
                                    deploy_result = await self.deploy_policy(
                                        result["policy_path"],
                                        result["version"]
                                    )
                                    
                                    if deploy_result["status"] == "success":
                                        # Compare with previous version
                                        if self.current_version > 1:
                                            comparison = await self.compare_versions(
                                                self.current_version - 1,
                                                self.current_version
                                            )
                                            
                                            # Check if new version is worse
                                            if comparison["status"] == "success":
                                                comp_data = comparison["comparison"]
                                                if comp_data.get("policy2_win_rate", 0) < 0.45:
                                                    # New policy is worse, rollback
                                                    await self.rollback_policy(self.current_version - 1)
                        
                        last_position = f.tell()
                
                # Check every 60 seconds
                await asyncio.sleep(60)
                
            except Exception as e:
                logger.error(f"Monitor loop error: {e}")
                await asyncio.sleep(60)


async def main():
    """Main entry point"""
    learner = ContinuousLearner()
    await learner.monitor_and_retrain_loop()


if __name__ == "__main__":
    # Create experiments directory
    Path("experiments").mkdir(exist_ok=True)
    asyncio.run(main())