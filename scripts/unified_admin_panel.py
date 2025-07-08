#!/usr/bin/env python3
"""
SentientOS Unified Admin Panel
Single dashboard for all monitoring and control
"""

import os
import sys
import json
import asyncio
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional
import logging
from collections import deque, defaultdict

# Install dependencies if missing
try:
    import aiofiles
    from aiohttp import web
    import aiohttp_cors
    import psutil
except ImportError:
    print("Installing required packages...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "aiohttp", "aiohttp-cors", "aiofiles", "psutil"])
    import aiofiles
    from aiohttp import web
    import aiohttp_cors
    import psutil

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class UnifiedDashboard:
    """Single admin panel for all SentientOS monitoring"""
    
    def __init__(self, port: int = 8081):
        self.port = port
        self.app = web.Application()
        self.logs_dir = Path("logs")
        
        # Caches for performance
        self.metrics_cache = {}
        self.last_update = datetime.now()
        
        self.setup_routes()
        self.setup_cors()
    
    def convert_to_pst(self, timestamp_str: str) -> str:
        """Convert UTC timestamp to PST (simplified without zoneinfo)"""
        try:
            if timestamp_str.endswith('Z'):
                timestamp_str = timestamp_str[:-1]
            dt = datetime.fromisoformat(timestamp_str)
            # Simple PST conversion (UTC-8)
            pst_dt = dt - timedelta(hours=8)
            return pst_dt.isoformat()
        except:
            return timestamp_str
    
    def setup_routes(self):
        """Configure all API routes"""
        self.app.router.add_get('/', self.index)
        self.app.router.add_get('/api/dashboard/all', self.get_all_data)
        self.app.router.add_get('/api/system/status', self.get_system_status)
        self.app.router.add_get('/api/activity/recent', self.get_recent_activity)
        self.app.router.add_post('/api/goal/inject', self.inject_goal)
    
    def setup_cors(self):
        """Enable CORS for all origins"""
        cors = aiohttp_cors.setup(self.app, defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*"
            )
        })
        
        for route in list(self.app.router.routes()):
            cors.add(route)
    
    async def index(self, request):
        """Serve the admin panel HTML"""
        html_content = """
<!DOCTYPE html>
<html>
<head>
    <title>SentientOS Admin Panel</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0a;
            color: #e0e0e0;
            line-height: 1.6;
        }
        
        .header {
            background: #1a1a1a;
            border-bottom: 2px solid #00ff88;
            padding: 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        h1 {
            color: #00ff88;
            font-size: 2em;
        }
        
        .container {
            max-width: 1600px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .card {
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
            position: relative;
            overflow: hidden;
        }
        
        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
        }
        
        .card-title {
            color: #00ff88;
            font-size: 1.2em;
            font-weight: bold;
        }
        
        .metric-value {
            font-size: 2.5em;
            font-weight: bold;
            color: #00ff88;
            margin: 10px 0;
        }
        
        .activity-feed {
            background: #0d0d0d;
            border-radius: 4px;
            padding: 15px;
            max-height: 400px;
            overflow-y: auto;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
        }
        
        .log-entry {
            margin-bottom: 8px;
            padding: 8px;
            border-left: 3px solid #444;
            background: rgba(255, 255, 255, 0.02);
        }
        
        .log-success { border-left-color: #00ff88; }
        .log-error { border-left-color: #ff4444; }
        
        .status-indicator {
            display: inline-block;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            margin-right: 5px;
        }
        
        .status-active { background: #00ff88; }
        .status-inactive { background: #ff4444; }
        
        .btn {
            padding: 10px 20px;
            background: #00ff88;
            color: #000;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-weight: bold;
        }
        
        .btn:hover {
            background: #00cc66;
        }
        
        .goal-input {
            width: 100%;
            padding: 10px;
            background: #0d0d0d;
            border: 1px solid #333;
            color: #e0e0e0;
            border-radius: 4px;
            margin-bottom: 10px;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>🧠 SentientOS Admin Panel</h1>
        <div id="last-update"></div>
    </div>
    
    <div class="container">
        <div class="grid">
            <div class="card">
                <div class="card-header">
                    <div class="card-title">System Status</div>
                </div>
                <div id="system-metrics">
                    <div>CPU: <span class="metric-value" id="cpu-percent">-</span></div>
                    <div>Memory: <span class="metric-value" id="memory-percent">-</span></div>
                    <div>Disk: <span class="metric-value" id="disk-percent">-</span></div>
                </div>
            </div>
            
            <div class="card">
                <div class="card-header">
                    <div class="card-title">Service Status</div>
                </div>
                <div id="service-status">Loading...</div>
            </div>
            
            <div class="card">
                <div class="card-header">
                    <div class="card-title">Goal Injection</div>
                </div>
                <input type="text" class="goal-input" id="goal-input" 
                       placeholder="Enter a goal (e.g., 'Check disk usage')">
                <button class="btn" onclick="injectCustomGoal()">Inject Goal</button>
            </div>
        </div>
        
        <div class="card">
            <div class="card-header">
                <div class="card-title">Recent Activity</div>
            </div>
            <div class="activity-feed" id="activity-feed">
                Loading activity...
            </div>
        </div>
    </div>
    
    <script>
        let refreshInterval;
        
        async function refreshData() {
            try {
                const response = await fetch('/api/dashboard/all');
                const data = await response.json();
                
                // Update system metrics
                document.getElementById('cpu-percent').textContent = data.system.cpu_percent.toFixed(1) + '%';
                document.getElementById('memory-percent').textContent = data.system.memory_percent.toFixed(1) + '%';
                document.getElementById('disk-percent').textContent = data.system.disk_usage.toFixed(1) + '%';
                
                // Update service status
                const serviceHtml = data.processes.map(p => `
                    <div>
                        <span class="status-indicator status-${p.status === 'running' ? 'active' : 'inactive'}"></span>
                        ${p.name}: ${p.status}
                    </div>
                `).join('');
                document.getElementById('service-status').innerHTML = serviceHtml;
                
                // Update activity feed
                const activityHtml = data.activity.slice(0, 20).map(a => `
                    <div class="log-entry log-${a.success ? 'success' : 'error'}">
                        <strong>${new Date(a.timestamp).toLocaleTimeString()}</strong> - 
                        ${a.goal.substring(0, 50)}...
                        (reward: ${a.reward.toFixed(2)})
                    </div>
                `).join('');
                document.getElementById('activity-feed').innerHTML = activityHtml || 'No recent activity';
                
                // Update timestamp
                document.getElementById('last-update').textContent = 
                    'Last update: ' + new Date().toLocaleTimeString();
                    
            } catch (error) {
                console.error('Failed to refresh data:', error);
            }
        }
        
        async function injectCustomGoal() {
            const goal = document.getElementById('goal-input').value;
            if (!goal) return;
            
            try {
                const response = await fetch('/api/goal/inject', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ goal })
                });
                
                if (response.ok) {
                    alert('Goal injected successfully!');
                    document.getElementById('goal-input').value = '';
                    refreshData();
                } else {
                    alert('Failed to inject goal');
                }
            } catch (error) {
                console.error('Failed to inject goal:', error);
                alert('Error injecting goal');
            }
        }
        
        // Start auto-refresh
        refreshData();
        refreshInterval = setInterval(refreshData, 5000);
        
        // Cleanup on page unload
        window.addEventListener('beforeunload', () => {
            if (refreshInterval) clearInterval(refreshInterval);
        });
    </script>
</body>
</html>
"""
        return web.Response(text=html_content, content_type='text/html')
    
    async def get_all_data(self, request):
        """Get all dashboard data in one request"""
        data = {
            "system": await self._get_system_metrics(),
            "processes": await self._get_process_status(),
            "activity": await self._get_recent_activity(limit=20),
            "timestamp": datetime.now().isoformat()
        }
        return web.json_response(data)
    
    async def get_system_status(self, request):
        """Get current system status"""
        return web.json_response(await self._get_system_metrics())
    
    async def get_recent_activity(self, request):
        """Get recent activity feed"""
        limit = int(request.query.get('limit', 50))
        activity = await self._get_recent_activity(limit)
        return web.json_response(activity)
    
    async def inject_goal(self, request):
        """Inject a goal into the system"""
        try:
            data = await request.json()
            goal = data.get('goal', '').strip()
            
            if not goal:
                return web.json_response({"error": "Goal cannot be empty"}, status=400)
            
            injection_entry = {
                "goal": goal,
                "source": "admin_panel",
                "timestamp": datetime.now().isoformat() + 'Z',
                "reasoning": "Manual injection from admin panel",
                "priority": "medium",
                "injected": True,
                "processed": False
            }
            
            injection_file = self.logs_dir / "goal_injections.jsonl"
            async with aiofiles.open(injection_file, 'a') as f:
                await f.write(json.dumps(injection_entry) + '\n')
            
            return web.json_response({"status": "success", "goal": goal})
        
        except Exception as e:
            return web.json_response({"error": str(e)}, status=500)
    
    # Helper methods
    async def _get_system_metrics(self) -> Dict:
        """Get current system resource metrics"""
        try:
            return {
                "cpu_percent": psutil.cpu_percent(interval=0.1),
                "memory_percent": psutil.virtual_memory().percent,
                "disk_usage": psutil.disk_usage('/').percent,
                "process_count": len(psutil.pids()),
                "timestamp": datetime.now().isoformat()
            }
        except:
            return {"error": "Unable to get system metrics"}
    
    async def _get_process_status(self) -> List[Dict]:
        """Get status of key processes"""
        processes = [
            {"name": "Goal Processor", "file": "fast_goal"},
            {"name": "LLM Observer", "file": "llm_observer"},
            {"name": "Reflective Analyzer", "file": "reflective"},
            {"name": "Admin Panel", "file": "admin_panel"},
        ]
        
        status_list = []
        
        for proc in processes:
            # Check if process is running
            is_running = False
            try:
                for p in psutil.process_iter(['pid', 'name', 'cmdline']):
                    if p.info['cmdline'] and proc['file'] in ' '.join(p.info['cmdline']):
                        is_running = True
                        break
            except:
                pass
            
            status_list.append({
                "name": proc['name'],
                "status": "running" if is_running else "stopped"
            })
        
        return status_list
    
    async def _get_recent_activity(self, limit: int = 50) -> List[Dict]:
        """Get recent activity from logs"""
        activity = []
        
        # Read from goal log
        log_file = self.logs_dir / f"fast_goal_log_{datetime.now():%Y%m%d}.jsonl"
        if log_file.exists():
            try:
                async with aiofiles.open(log_file, 'r') as f:
                    lines = await f.readlines()
                    for line in lines[-limit:]:
                        try:
                            entry = json.loads(line.strip())
                            activity.append(entry)
                        except:
                            continue
            except:
                pass
        
        # Sort by timestamp
        activity.sort(key=lambda x: x.get('timestamp', ''), reverse=True)
        return activity[:limit]
    
    async def run(self):
        """Start the dashboard server"""
        runner = web.AppRunner(self.app)
        await runner.setup()
        site = web.TCPSite(runner, '0.0.0.0', self.port)
        await site.start()
        
        logger.info(f"🎛️  Unified Admin Panel started on http://0.0.0.0:{self.port}")
        logger.info(f"   Access from: http://localhost:{self.port}")
        
        # Keep running
        while True:
            await asyncio.sleep(3600)


async def main():
    """Entry point"""
    dashboard = UnifiedDashboard(port=8081)
    await dashboard.run()


if __name__ == "__main__":
    asyncio.run(main())