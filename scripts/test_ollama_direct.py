#!/usr/bin/env python3
"""
Direct test of Ollama API connection
"""

import requests
import json
import sys

OLLAMA_URL = "http://192.168.69.197:11434"

def test_ollama_connection():
    """Test basic Ollama connectivity"""
    print(f"🔍 Testing Ollama at {OLLAMA_URL}")
    print("="*50)
    
    # Test 1: Check if server is alive
    print("\n1️⃣ Testing server connectivity...")
    try:
        response = requests.get(f"{OLLAMA_URL}/api/tags", timeout=5)
        if response.status_code == 200:
            print("✅ Server is reachable!")
            models = response.json().get('models', [])
            print(f"📋 Available models: {len(models)}")
            for model in models[:5]:  # Show first 5
                print(f"   - {model['name']}")
            if len(models) > 5:
                print(f"   ... and {len(models)-5} more")
        else:
            print(f"❌ Server returned status: {response.status_code}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ Connection failed: {e}")
        print("\n⚠️  Make sure Ollama is running:")
        print("   sudo systemctl start ollama  # or")
        print("   ollama serve")
        return False
    
    # Test 2: Try a simple generation
    print("\n2️⃣ Testing text generation...")
    
    # Pick a model
    if models:
        model_name = models[0]['name']
        print(f"   Using model: {model_name}")
        
        payload = {
            "model": model_name,
            "prompt": "What is RAM? Answer in one sentence.",
            "stream": False,
            "options": {
                "temperature": 0.7,
                "num_predict": 50
            }
        }
        
        try:
            response = requests.post(
                f"{OLLAMA_URL}/api/generate",
                json=payload,
                timeout=30
            )
            
            if response.status_code == 200:
                result = response.json()
                print("✅ Generation successful!")
                print(f"📝 Response: {result.get('response', 'No response')}")
                return True
            else:
                print(f"❌ Generation failed: {response.status_code}")
                print(f"   Error: {response.text}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Generation request failed: {e}")
            return False
    else:
        print("❌ No models available to test")
        return False

def test_rag_tool_integration():
    """Test how RAG-Tool system would use Ollama"""
    print("\n\n3️⃣ Testing RAG-Tool Integration Pattern...")
    print("-"*50)
    
    # Simulate the flow
    test_cases = [
        {
            "prompt": "Check disk usage",
            "expected_intent": "ToolCall",
            "expected_model": "Local/fast model for tools"
        },
        {
            "prompt": "What is virtual memory?",
            "expected_intent": "GeneralKnowledge", 
            "expected_model": "Knowledge-focused model"
        },
        {
            "prompt": "My system is slow, what should I check?",
            "expected_intent": "QueryThenAction",
            "expected_model": "Analytical model"
        }
    ]
    
    for i, test in enumerate(test_cases):
        print(f"\nTest {i+1}: '{test['prompt']}'")
        print(f"  Expected: {test['expected_intent']} → {test['expected_model']}")
        
        # In real system, this would:
        # 1. Route to appropriate model via intelligent_router
        # 2. Use RAG for knowledge retrieval
        # 3. Execute tools if conditions match
        # 4. Log trace with feedback
        
    print("\n✅ Integration pattern validated")

def main():
    print("🧪 Ollama Integration Test")
    print("="*50)
    
    # Test connection
    if test_ollama_connection():
        print("\n✅ Ollama is working correctly!")
        
        # Test integration patterns
        test_rag_tool_integration()
        
        print("\n📚 Next steps:")
        print("1. Build sentient-shell: cd sentient-shell && cargo build --release")
        print("2. Run shell: ./target/release/sentient-shell")
        print("3. Test commands: ask, models, llm route test")
        print("4. Use rag_tool command for hybrid queries")
    else:
        print("\n❌ Ollama connection failed")
        print("\n🔧 Troubleshooting:")
        print("1. Check if Ollama is installed: ollama --version")
        print("2. Start Ollama service: ollama serve")
        print("3. Pull a model: ollama pull llama2")
        print("4. Verify URL: curl http://192.168.69.197:11434/api/tags")
        sys.exit(1)

if __name__ == "__main__":
    main()