#!/usr/bin/env python3
"""
Test DeepSeek v2 integration with Ollama
"""

import requests
import json
import time

OLLAMA_URL = "http://192.168.69.197:11434"
DEEPSEEK_MODEL = "deepseek-v2:16b"

def test_deepseek_generation():
    """Test DeepSeek v2 model directly"""
    print(f"🧪 Testing {DEEPSEEK_MODEL} on Ollama")
    print("="*60)
    
    test_prompts = [
        {
            "type": "General Knowledge",
            "prompt": "What is the difference between RAM and storage? Answer concisely.",
            "max_tokens": 100
        },
        {
            "type": "Tool Decision",
            "prompt": "User says: 'My disk is full'. Should I run disk_info tool? Answer yes or no with brief reason.",
            "max_tokens": 50
        },
        {
            "type": "Code Generation",
            "prompt": "Write a Python function to check disk usage. Just the function, no explanation.",
            "max_tokens": 200
        },
        {
            "type": "Analysis",
            "prompt": "System shows: Memory 95% used, CPU 20%, Disk 85%. What's the main issue?",
            "max_tokens": 100
        }
    ]
    
    for i, test in enumerate(test_prompts, 1):
        print(f"\n📝 Test {i}: {test['type']}")
        print(f"Prompt: {test['prompt']}")
        print("-" * 40)
        
        payload = {
            "model": DEEPSEEK_MODEL,
            "prompt": test['prompt'],
            "stream": False,
            "options": {
                "temperature": 0.7,
                "num_predict": test['max_tokens']
            }
        }
        
        try:
            start_time = time.time()
            response = requests.post(
                f"{OLLAMA_URL}/api/generate",
                json=payload,
                timeout=30
            )
            duration = (time.time() - start_time) * 1000  # ms
            
            if response.status_code == 200:
                result = response.json()
                print(f"✅ Response ({duration:.0f}ms):")
                print(result.get('response', 'No response'))
                
                # Check if this would trigger tool execution
                if test['type'] == "Tool Decision":
                    response_text = result.get('response', '').lower()
                    if 'yes' in response_text:
                        print("🔧 → Would trigger disk_info tool")
            else:
                print(f"❌ Error: {response.status_code}")
                print(response.text)
                
        except Exception as e:
            print(f"❌ Request failed: {e}")

def simulate_rag_tool_pipeline():
    """Simulate the complete RAG-Tool pipeline with DeepSeek"""
    print("\n\n🔄 Simulating RAG-Tool Pipeline with DeepSeek")
    print("="*60)
    
    # Example: User asks about disk space
    user_query = "My system feels slow and disk might be full. What should I check?"
    
    print(f"👤 User: {user_query}")
    
    # Step 1: Intent Detection (using DeepSeek)
    print("\n1️⃣ Intent Detection...")
    intent_prompt = f"""Classify this user query into one of these intents:
- ToolCall: Direct tool execution request
- GeneralKnowledge: Information query
- Analysis: System analysis request
- QueryThenAction: Information followed by action

Query: "{user_query}"
Intent:"""
    
    payload = {
        "model": DEEPSEEK_MODEL,
        "prompt": intent_prompt,
        "stream": False,
        "options": {"temperature": 0.3, "num_predict": 20}
    }
    
    try:
        response = requests.post(f"{OLLAMA_URL}/api/generate", json=payload, timeout=10)
        if response.status_code == 200:
            intent = response.json().get('response', '').strip()
            print(f"   Detected: {intent}")
    except:
        intent = "Analysis"
        print(f"   Fallback: {intent}")
    
    # Step 2: RAG Query (simulated)
    print("\n2️⃣ RAG Retrieval...")
    print("   Found: System performance troubleshooting guide")
    print("   Found: Disk space management documentation")
    
    # Step 3: Tool Decision
    print("\n3️⃣ Tool Execution Decision...")
    tool_prompt = f"""Based on this situation: "{user_query}"
    
Should we execute these tools? Reply with yes/no for each:
- disk_info: Check disk usage
- memory_usage: Check RAM usage
- process_list: List running processes

Format: tool_name: yes/no"""
    
    payload = {
        "model": DEEPSEEK_MODEL,
        "prompt": tool_prompt,
        "stream": False,
        "options": {"temperature": 0.3, "num_predict": 50}
    }
    
    try:
        response = requests.post(f"{OLLAMA_URL}/api/generate", json=payload, timeout=10)
        if response.status_code == 200:
            tool_decision = response.json().get('response', '')
            print(f"   Decision: {tool_decision}")
            
            # Parse decisions
            if 'disk_info: yes' in tool_decision.lower():
                print("\n🔧 Executing: disk_info")
                print("   Output: /dev/sda1 85% used (17GB/20GB)")
    except Exception as e:
        print(f"   Error: {e}")
    
    # Step 4: Final Response
    print("\n4️⃣ Generating Final Response...")
    print("   Combining RAG knowledge + tool results...")
    print("\n📤 Final Response to User:")
    print("Your disk is 85% full, which can cause system slowdowns. ")
    print("I recommend cleaning up old files or moving data to external storage.")
    print("The disk_info tool shows 17GB used out of 20GB total.")

def main():
    # Test direct generation
    test_deepseek_generation()
    
    # Simulate pipeline
    simulate_rag_tool_pipeline()
    
    print("\n\n✅ DeepSeek v2 integration test complete!")
    print("\n📚 Next steps:")
    print("1. Build shell with updated config: cd sentient-shell && cargo build --release")
    print("2. Test with shell: ./target/release/sentient-shell")
    print("3. Try commands:")
    print("   - ask 'What is virtual memory?'")
    print("   - rag_tool 'Check my disk usage' --explain")
    print("   - llm route test 'Write a Python script'")

if __name__ == "__main__":
    main()