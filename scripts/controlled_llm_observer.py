#!/usr/bin/env python3
"""
Controlled LLM Observer - Injects AI-generated goals at regular intervals
"""

import json
import asyncio
import aiohttp
from datetime import datetime
from pathlib import Path
import random

class ControlledLLMObserver:
    """Periodically queries LLM for new goals"""
    
    def __init__(self, interval: int = 30):
        self.interval = interval
        self.logs_dir = Path("logs")
        self.logs_dir.mkdir(exist_ok=True)
        self.ollama_url = "http://192.168.69.197:11434"
        
        # Predefined useful goals for fallback
        self.fallback_goals = [
            "Monitor disk I/O activity and report any anomalies",
            "Check current memory usage and identify top consumers",
            "Analyze network connections for unusual patterns",
            "Review system logs for recent errors or warnings",
            "Measure CPU load and identify resource-intensive processes",
            "Verify all critical services are running properly",
            "Check disk space usage across all mounted filesystems",
            "Monitor system uptime and last reboot time",
            "Analyze process count trends over time",
            "Identify potential performance bottlenecks"
        ]
    
    async def query_llm(self, prompt: str) -> str:
        """Query Ollama for goal generation"""
        try:
            async with aiohttp.ClientSession() as session:
                payload = {
                    "model": "deepseek-v2:16b",
                    "prompt": prompt,
                    "stream": False,
                    "options": {
                        "temperature": 0.7,
                        "max_tokens": 100
                    }
                }
                
                async with session.post(
                    f"{self.ollama_url}/api/generate",
                    json=payload,
                    timeout=aiohttp.ClientTimeout(total=20)
                ) as response:
                    if response.status == 200:
                        data = await response.json()
                        return data.get('response', '').strip()
        except:
            pass
        
        return ""
    
    async def generate_goal(self) -> str:
        """Generate a new goal using LLM or fallback"""
        prompt = """You are a system monitoring AI. Generate ONE specific, actionable goal for monitoring system health.
Focus on: disk usage, memory, CPU, network, processes, or logs.
Be specific and technical. Output only the goal, nothing else.
Example: "Check disk I/O patterns for the root filesystem"
Goal:"""
        
        # Try LLM first
        goal = await self.query_llm(prompt)
        
        # Fallback to predefined goals if LLM fails
        if not goal or len(goal) < 10:
            goal = random.choice(self.fallback_goals)
            print("   📋 Using fallback goal")
        else:
            print("   🤖 LLM generated goal")
        
        return goal
    
    def inject_goal(self, goal: str):
        """Inject goal into the system"""
        injection_entry = {
            "goal": goal,
            "source": "llm_observer",
            "timestamp": datetime.now().isoformat() + 'Z',
            "reasoning": "AI-generated monitoring goal",
            "priority": "medium",
            "injected": True,
            "processed": False
        }
        
        injection_file = self.logs_dir / "goal_injections.jsonl"
        try:
            with open(injection_file, 'a') as f:
                f.write(json.dumps(injection_entry) + '\n')
            
            print(f"   ✓ Injected: {goal[:60]}...")
            
            # Also log to LLM activity
            self.log_activity(injection_entry)
        except Exception as e:
            print(f"   ❌ Failed to inject goal: {e}")
    
    def log_activity(self, entry: dict):
        """Log LLM activity"""
        log_file = self.logs_dir / f"llm_activity_{datetime.now():%Y%m%d}.jsonl"
        try:
            with open(log_file, 'a') as f:
                f.write(json.dumps(entry) + '\n')
        except:
            pass
    
    async def run(self):
        """Main loop"""
        print("🤖 Controlled LLM Observer started")
        print(f"   Injection interval: {self.interval}s")
        print(f"   Ollama URL: {self.ollama_url}")
        
        # Stagger start by 15 seconds to avoid collision with processor heartbeat
        await asyncio.sleep(15)
        
        while True:
            try:
                print(f"\n🔮 Generating new goal at {datetime.now():%H:%M:%S}...")
                
                # Generate and inject goal
                goal = await self.generate_goal()
                if goal:
                    self.inject_goal(goal)
                
                # Wait for next cycle
                await asyncio.sleep(self.interval)
                
            except KeyboardInterrupt:
                print("\n👋 Shutting down LLM observer")
                break
            except Exception as e:
                print(f"\n❌ Error in LLM observer: {e}")
                await asyncio.sleep(self.interval)


async def main():
    """Entry point"""
    observer = ControlledLLMObserver(interval=30)
    await observer.run()


if __name__ == "__main__":
    asyncio.run(main())