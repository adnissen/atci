# Integrating Hugging Face Models with Elixir Applications

## Overview

You have several options for integrating Hugging Face models into your Elixir application. Each approach has different trade-offs in terms of complexity, performance, and maintenance requirements.

## Option 1: Direct Python Integration with Pythonx (Recommended for Rapid Prototyping)

**Pythonx** is a new Elixir library that embeds the Python interpreter directly within the Erlang VM, allowing seamless data conversion between Elixir and Python.

### Advantages:
- Direct access to the entire Python ML ecosystem
- Automatic data conversion between Elixir and Python
- Can cache loaded models in memory for better performance
- Minimal setup required
- Can run any Hugging Face model immediately

### Disadvantages:
- Python's Global Interpreter Lock (GIL) limits concurrency
- Must be used carefully in production (single process recommended)
- Adds Python as a runtime dependency

### Example Usage:
```elixir
Mix.install([
  {:pythonx, "~> 0.4.0"}
])

# Initialize Python environment with dependencies
Pythonx.uv_init("""
[project]
name = "project"
version = "0.0.0"
requires-python = "==3.13.*"
dependencies = [
  "transformers==4.36.0",
  "torch==2.1.0",
  "numpy==1.24.0"
]
""")

# Load model once and cache it
{_, globals} = Pythonx.eval("""
from transformers import pipeline

# Load your specific model
classifier = pipeline("text-classification", model="your-model-name")

def classify_text(text):
    result = classifier(text)
    return result[0]['label'], result[0]['score']
""", %{})

# Use the cached model for predictions
{result, _} = Pythonx.eval("""
classify_text("Your input text here")
""", globals)

IO.inspect(result)
```

## Option 2: Elixir Native ML Stack (Nx/Bumblebee)

**Bumblebee** provides pre-trained neural network models that run on Nx (Numerical Elixir), offering a pure Elixir solution.

### Advantages:
- Pure Elixir implementation
- Excellent concurrency support
- No external dependencies
- Integrates well with Phoenix/LiveView
- Production-ready with `Nx.Serving`

### Disadvantages:
- Limited model selection compared to Hugging Face
- May require model conversion
- Newer ecosystem with fewer resources

### Example Usage:
```elixir
# In your mix.exs
defp deps do
  [
    {:bumblebee, "~> 0.4.0"},
    {:nx, "~> 0.6.0"},
    {:exla, "~> 0.6.0"} # For GPU/CPU acceleration
  ]
end

# In your application
{:ok, model_info} = Bumblebee.load_model({:hf, "microsoft/DialoGPT-medium"})
{:ok, tokenizer} = Bumblebee.load_tokenizer({:hf, "microsoft/DialoGPT-medium"})
{:ok, generation_config} = Bumblebee.load_generation_config({:hf, "microsoft/DialoGPT-medium"})

serving = Bumblebee.Text.generation(model_info, tokenizer, generation_config)

# Use in your application
Nx.Serving.run(serving, "Hello, how are you?")
```

## Option 3: HTTP API Approach

Deploy your Hugging Face model as a separate HTTP service and communicate via REST API.

### Advantages:
- Complete separation of concerns
- Can scale ML service independently
- Multiple Elixir processes can use the service concurrently
- Can use any ML framework/language for the service

### Disadvantages:
- Network latency
- Additional infrastructure complexity
- Need to manage separate service deployment

### Implementation:

**Python Service (FastAPI example):**
```python
from fastapi import FastAPI
from transformers import pipeline

app = FastAPI()
classifier = pipeline("text-classification", model="your-model-name")

@app.post("/predict")
async def predict(text: str):
    result = classifier(text)
    return {"label": result[0]['label'], "score": result[0]['score']}
```

**Elixir Client:**
```elixir
defmodule MLClient do
  def predict(text) do
    body = Jason.encode!(%{text: text})
    
    case HTTPoison.post("http://localhost:8000/predict", body, [{"Content-Type", "application/json"}]) do
      {:ok, %HTTPoison.Response{status_code: 200, body: response_body}} ->
        Jason.decode!(response_body)
      {:error, reason} ->
        {:error, reason}
    end
  end
end
```

## Option 4: NIFs (Native Implemented Functions)

Create native extensions that interface with C/C++ ML libraries or Python C extensions.

### Advantages:
- Maximum performance
- Fine-grained control
- Can integrate with existing C/C++ ML libraries

### Disadvantages:
- High complexity
- Risk of crashes/memory leaks
- Significant development time
- Requires C/C++ expertise

### Tools:
- **Rustler**: Write NIFs in Rust (safer than C)
- **Zigler**: Write NIFs in Zig
- Direct C NIFs using `erl_nif.h`

## Option 5: Port/System Commands

Execute Python scripts as separate OS processes.

### Advantages:
- Process isolation (crashes don't affect Elixir)
- Can run multiple Python processes
- Simple to implement

### Disadvantages:
- High overhead for each prediction
- Process startup costs
- Data serialization overhead

### Example:
```elixir
defmodule MLRunner do
  def predict(text) do
    case System.cmd("python", ["predict.py", text]) do
      {result, 0} -> 
        Jason.decode!(result)
      {error, _} -> 
        {:error, error}
    end
  end
end
```

## Option 6: Model Context Protocol (MCP) Integration

Use Hermes MCP to create structured communication between your Elixir app and Python ML tools.

### Advantages:
- Structured tool-based interaction
- Can dynamically discover ML capabilities
- Good for complex ML workflows

### Disadvantages:
- Additional complexity
- Newer technology with limited examples
- Requires understanding of MCP protocol

## Recommendations by Use Case

### For Rapid Prototyping and Development:
- **Use Pythonx** - Fastest way to get any Hugging Face model running in Elixir

### For Production Applications:
- **HTTP API approach** - Best for scalability and reliability
- **Bumblebee/Nx** - If your required models are available

### For High-Performance Requirements:
- **NIFs with Rustler** - If you need maximum performance and control

### For Your Current Phoenix Application:
Given that you have an existing Phoenix application (based on your `mix.exs`), I'd recommend starting with **Pythonx** for experimentation, then moving to either:
1. **HTTP API** for production deployment
2. **Bumblebee** if your model is supported

## Getting Started with Your Current Setup

Add Pythonx to your existing Phoenix application:

```elixir
# In mix.exs
defp deps do
  [
    {:phoenix, "~> 1.7"},
    {:phoenix_html, "~> 4.0"},
    {:phoenix_live_view, "~> 0.20"},
    {:plug_cowboy, "~> 2.7"},
    {:gettext, ">= 0.24.0"},
    {:jason, "~> 1.4"},
    {:uuid, "~> 1.1"},
    {:pythonx, "~> 0.4.0"}  # Add this
  ]
end
```

Then create a module to handle ML predictions:

```elixir
defmodule Autotranscript.ML do
  def initialize_model do
    Pythonx.uv_init("""
    [project]
    name = "autotranscript-ml"
    version = "0.0.0"
    requires-python = "==3.13.*"
    dependencies = [
      "transformers==4.36.0",
      "torch==2.1.0"
    ]
    """)
    
    Pythonx.eval("""
    from transformers import pipeline
    
    # Replace with your specific Hugging Face model
    model = pipeline("automatic-speech-recognition", model="openai/whisper-base")
    
    def transcribe_audio(audio_path):
        result = model(audio_path)
        return result["text"]
    """, %{})
  end
  
  def transcribe(audio_path, globals) do
    {result, _} = Pythonx.eval("""
    transcribe_audio(audio_path)
    """, Map.put(globals, "audio_path", audio_path))
    
    Pythonx.decode(result)
  end
end
```

This approach will let you quickly integrate any Hugging Face model into your existing Phoenix application while maintaining the flexibility to migrate to other approaches as your needs evolve.