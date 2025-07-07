#!/usr/bin/env python3
"""
List all available Ollama models and find DeepSeek v2
"""

import requests
import json

OLLAMA_URL = "http://192.168.69.197:11434"

def list_all_models():
    """List all available models in Ollama"""
    try:
        response = requests.get(f"{OLLAMA_URL}/api/tags", timeout=5)
        if response.status_code == 200:
            data = response.json()
            models = data.get('models', [])
            
            print(f"📋 Found {len(models)} models in Ollama:\n")
            
            # Look for DeepSeek models
            deepseek_models = []
            
            for model in models:
                model_name = model['name']
                size = model.get('size', 0) / (1024**3)  # Convert to GB
                
                # Check if it's a DeepSeek model
                if 'deepseek' in model_name.lower():
                    deepseek_models.append(model_name)
                    print(f"🎯 {model_name} ({size:.1f}GB) - DEEPSEEK MODEL")
                else:
                    print(f"   {model_name} ({size:.1f}GB)")
            
            if deepseek_models:
                print(f"\n✅ Found {len(deepseek_models)} DeepSeek model(s):")
                for dm in deepseek_models:
                    print(f"   - {dm}")
                    
                # Find v2 specifically
                v2_models = [m for m in deepseek_models if 'v2' in m.lower() or '2' in m]
                if v2_models:
                    print(f"\n🎯 DeepSeek V2 model: {v2_models[0]}")
                    return v2_models[0]
            else:
                print("\n❌ No DeepSeek models found!")
                print("\n💡 To install DeepSeek v2, run:")
                print("   ollama pull deepseek-v2.5")
                print("   ollama pull deepseek-coder-v2")
                
            return None
            
        else:
            print(f"❌ Failed to get models: {response.status_code}")
            return None
            
    except Exception as e:
        print(f"❌ Error connecting to Ollama: {e}")
        return None

def main():
    print("🔍 Searching for DeepSeek v2 in Ollama...")
    print("="*50)
    
    deepseek_model = list_all_models()
    
    if deepseek_model:
        print(f"\n✅ Use this model name in your configuration: {deepseek_model}")
    else:
        print("\n💡 Available DeepSeek models to pull:")
        print("   - deepseek-v2.5:latest")
        print("   - deepseek-v2.5:16b-lite-chat-q4_0")
        print("   - deepseek-coder-v2:latest")
        print("   - deepseek-coder-v2:16b-lite-instruct-q4_0")

if __name__ == "__main__":
    main()