#!/usr/bin/env python3
"""
Fast Goal Processor - Core execution engine for SentientOS
Processes goals every 5 seconds with real command execution
"""

import json
import time
import subprocess
import asyncio
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import random
import os

class FastGoalProcessor:
    """Processes goals quickly with actual command execution"""
    
    def __init__(self, check_interval: int = 5):
        self.check_interval = check_interval
        self.logs_dir = Path("logs")
        self.logs_dir.mkdir(exist_ok=True)
        self.processed_goals = set()
        self.last_heartbeat = time.time()
        self.heartbeat_interval = 60  # seconds
        
    def goal_to_command(self, goal: str) -> str:
        """Convert goal to actual executable command"""
        goal_lower = goal.lower()
        
        # Disk activity/IO
        if 'disk' in goal_lower and any(word in goal_lower for word in ['activity', 'i/o', 'io', 'usage']):
            return "df -h | grep -E '^/dev/' | head -3 && echo '---' && iostat -d 1 2 2>/dev/null | tail -n +4 | awk 'NR>1 {print $1 \": Read \" $3 \" KB/s, Write \" $4 \" KB/s\"}' || echo 'Disk stats: iostat not available'"
        
        # Memory usage
        elif 'memory' in goal_lower and any(word in goal_lower for word in ['usage', 'check', 'free']):
            return "free -h | grep -E '^Mem:' | awk '{print \"Memory: Total \" $2 \", Used \" $3 \", Free \" $4 \", Available \" $7}'"
        
        # Network activity
        elif 'network' in goal_lower and any(word in goal_lower for word in ['activity', 'connections', 'traffic']):
            return "netstat -tunl 2>/dev/null | grep LISTEN | wc -l | xargs -I {} echo 'Active listeners: {}' && ss -s 2>/dev/null | grep 'TCP:' || echo 'Network stats unavailable'"
        
        # CPU usage
        elif 'cpu' in goal_lower and any(word in goal_lower for word in ['usage', 'load', 'check']):
            return "uptime | awk -F'load average:' '{print \"Load average:\" $2}' && top -bn1 | grep 'Cpu(s)' | head -1 || echo 'CPU stats unavailable'"
        
        # Process count
        elif 'process' in goal_lower and any(word in goal_lower for word in ['count', 'running', 'check']):
            return "ps aux | wc -l | xargs -I {} echo 'Total processes: {}'"
        
        # System health/uptime
        elif any(word in goal_lower for word in ['health', 'uptime', 'status']):
            return "uptime -p && echo '---' && df -h / | tail -1 | awk '{print \"Root disk: \" $5 \" used\"}' && free -h | grep Mem | awk '{print \"Memory: \" $3 \"/\" $2 \" used\"}'"
        
        # Log analysis
        elif 'log' in goal_lower and any(word in goal_lower for word in ['error', 'check', 'analyze']):
            return "find logs -name '*.log' -mtime -1 2>/dev/null | head -5 | xargs -I {} sh -c 'echo \"=== {} ===\" && tail -20 {} | grep -iE \"error|fail|critical\" | tail -5' || echo 'No recent errors in logs'"
        
        # Service status
        elif 'service' in goal_lower and any(word in goal_lower for word in ['status', 'check']):
            return "ps aux | grep -E 'goal|llm|reflect' | grep -v grep | wc -l | xargs -I {} echo 'SentientOS services running: {}'"
        
        # Default - echo the goal
        else:
            return f"echo 'Goal: {goal}'"
    
    def calculate_reward(self, output: str, success: bool) -> float:
        """Calculate reward based on command output"""
        if not success:
            return 0.0
        
        reward = 0.3  # Base reward for successful execution
        
        # Bonus for actual data
        if len(output) > 50:
            reward += 0.2
        
        # Bonus for structured output
        if ':' in output or '|' in output:
            reward += 0.2
        
        # Bonus for numeric data
        if any(c.isdigit() for c in output):
            reward += 0.2
        
        # Penalty for errors
        if 'error' in output.lower() or 'unavailable' in output.lower():
            reward -= 0.1
        
        return max(0.0, min(1.0, reward))
    
    def execute_command(self, command: str) -> Tuple[str, bool, float]:
        """Execute a command and return output, success, and execution time"""
        start_time = time.time()
        
        try:
            result = subprocess.run(
                command,
                shell=True,
                capture_output=True,
                text=True,
                timeout=10
            )
            
            execution_time = time.time() - start_time
            output = result.stdout if result.stdout else result.stderr
            success = result.returncode == 0
            
            return output.strip(), success, execution_time
        
        except subprocess.TimeoutExpired:
            return "Command timed out", False, 10.0
        except Exception as e:
            return f"Execution error: {str(e)}", False, time.time() - start_time
    
    def process_goal(self, goal_entry: Dict) -> Dict:
        """Process a single goal and return results"""
        goal = goal_entry.get('goal', '')
        command = self.goal_to_command(goal)
        
        print(f"\n🎯 Processing: {goal[:80]}...")
        print(f"   Command: {command[:80]}...")
        
        output, success, execution_time = self.execute_command(command)
        reward = self.calculate_reward(output, success)
        
        # Create result entry
        result = {
            'timestamp': datetime.now().isoformat() + 'Z',
            'goal': goal,
            'source': goal_entry.get('source', 'unknown'),
            'command': command,
            'output': output[:500],  # Limit output size
            'success': success,
            'reward': reward,
            'execution_time': execution_time
        }
        
        print(f"   ✓ Success: {success}, Reward: {reward:.2f}")
        
        return result
    
    def load_goals(self) -> List[Dict]:
        """Load unprocessed goals from injection file"""
        injection_file = self.logs_dir / "goal_injections.jsonl"
        if not injection_file.exists():
            return []
        
        goals = []
        try:
            with open(injection_file, 'r') as f:
                lines = f.readlines()
            
            # Process and clear the file
            with open(injection_file, 'w') as f:
                for line in lines:
                    try:
                        entry = json.loads(line.strip())
                        if not entry.get('processed', False):
                            goals.append(entry)
                            entry['processed'] = True
                        f.write(json.dumps(entry) + '\n')
                    except:
                        continue
        except:
            pass
        
        return goals
    
    def write_log(self, entry: Dict):
        """Write execution log"""
        log_file = self.logs_dir / f"fast_goal_log_{datetime.now():%Y%m%d}.jsonl"
        try:
            with open(log_file, 'a') as f:
                f.write(json.dumps(entry) + '\n')
        except:
            pass
    
    def heartbeat(self):
        """System heartbeat - inject health check goal"""
        current_time = time.time()
        if current_time - self.last_heartbeat >= self.heartbeat_interval:
            print("\n💓 Heartbeat - injecting system health check")
            
            health_goal = {
                'goal': 'Check system health and resource usage',
                'source': 'heartbeat',
                'timestamp': datetime.now().isoformat() + 'Z',
                'priority': 'low',
                'processed': False
            }
            
            # Inject directly
            injection_file = self.logs_dir / "goal_injections.jsonl"
            try:
                with open(injection_file, 'a') as f:
                    f.write(json.dumps(health_goal) + '\n')
            except:
                pass
            
            self.last_heartbeat = current_time
    
    async def run(self):
        """Main processing loop"""
        print("🚀 Fast Goal Processor started")
        print(f"   Check interval: {self.check_interval}s")
        print(f"   Heartbeat interval: {self.heartbeat_interval}s")
        print(f"   Logs directory: {self.logs_dir}")
        
        while True:
            try:
                # Load new goals
                goals = self.load_goals()
                
                if goals:
                    print(f"\n📥 Found {len(goals)} new goals")
                    
                    # Process each goal
                    for goal in goals:
                        result = self.process_goal(goal)
                        self.write_log(result)
                
                # Heartbeat check
                self.heartbeat()
                
                # Sleep until next check
                await asyncio.sleep(self.check_interval)
                
            except KeyboardInterrupt:
                print("\n👋 Shutting down goal processor")
                break
            except Exception as e:
                print(f"\n❌ Error in main loop: {e}")
                await asyncio.sleep(self.check_interval)


async def main():
    """Entry point"""
    processor = FastGoalProcessor(check_interval=5)
    await processor.run()


if __name__ == "__main__":
    asyncio.run(main())