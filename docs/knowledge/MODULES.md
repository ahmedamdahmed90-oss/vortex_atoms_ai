# Knowledge Modules Documentation

## Overview

Vortex Atoms AI includes a knowledge system with specialized modules that provide domain-specific information. Each module is compiled into a compact binary format for efficient storage and retrieval.

## Available Modules

### 1. Bio-Medical Module

**File**: `knowledge/bio_medical_module.tcz`
**Size**: ~7.6 KB
**Source**: `knowledge/bio_medical_module.source.md`

**Capabilities:**
- Human general medicine
- Clinical pharmacology
- Veterinary science
- Avian medicine
- Budgie genetics
- Finch genetics
- Hagoromo, Blackwing, Opaline, and Rainbow mutation probability logic

**Safety Boundary**: This module is for educational decision support only. It does not diagnose, prescribe, provide patient-specific dosing, replace licensed clinicians/veterinarians, or replace emergency care.

**Intent Detection**: The system detects biomedical and avian-genetics intents and routes them to Kernel_03 as `module = "bio.medical"`.

---

### 2. Engineering Expert Module

**File**: `knowledge/engineering_expert_module.tcz`
**Source**: `knowledge/engineering_expert_module.source.md`

**Capabilities:**
- Electronics repair
- PCB diagnostics
- Lithium battery recycling algorithms
- Metal detector schematic modeling
- Precious-metal process-safety analysis

**Safety Boundary**: Supports diagnostics, modeling, sorting algorithms, hazard recognition, compliance, and certified-refiner handoff logic. Excludes operational recipes, reagent concentrations, reaction conditions, or hands-on instructions for dangerous chemical extraction or unsafe lithium-cell processing.

**Intent Detection**: Engineering intents are routed to Kernel_03 as `module = "engineering.expert"`.

---

### 3. Finance & Math Module

**File**: `knowledge/finance_math_module.tcz`
**Source**: `knowledge/finance_math_module.source.md`

**Capabilities:**
- Macroeconomics and microeconomics
- Corporate finance
- Factory logistics
- Risk analysis
- Statistics and forecasting
- Strategic planning

**Intent Detection**: Finance and math intents are routed to Kernel_03 as `module = "finance.math"`.

---

### 4. Global Humanities Module

**File**: `knowledge/global_humanities_module.bin`
**Size**: ~6.5 KB
**Source**: `knowledge/global_humanities_module.source.md`

**Capabilities:**
- Grammar and translation
- World-language dictionary
- Global history
- Geography
- GIS/mapping

**Intent Detection**: Humanities intents are routed to Kernel_03 as `module = "global.humanities"`.

---

## Knowledge System Architecture

### Loading Flow

```
1. User submits query
        │
        ▼
2. Kernel_02 detects intent via vector similarity
        │
        ▼
3. Matching fragment descriptor found
        │
        ▼
4. Kernel_03 loads fragment via mmap (read-only)
        │
        ▼
5. Query answered from mapped data
        │
        ▼
6. Fragment explicitly dropped (std::mem::drop)
```

### Memory Management

- **Memory Mapping**: Knowledge files are loaded using `memmap2` for efficient memory usage
- **50 MiB Budget**: Each module is validated to be below the 50 MiB mapping budget
- **Explicit Cleanup**: Memory is explicitly dropped after each request to prevent leaks
- **Hot Cache**: Frequently requested tokenized data is kept in a hot token cache

### Knowledge Fragment Format

| Format | Extension | Description |
|--------|-----------|-------------|
| TCZ | .tcz | Compressed text with metadata |
| BIN | .bin | Raw binary knowledge data |
| Source | .source.md | Human-readable source |

---

## Adding a New Knowledge Module

### Step 1: Create Source File

Create a markdown file in the `knowledge/` directory:

```markdown
# My Knowledge Module

## Section 1
Content here...

## Section 2
More content...
```

### Step 2: Create Compilation Script

Create a Python script in `tools/compile_my_module.py`:

```python
#!/usr/bin/env python3
"""Compile my knowledge module."""

import sys
import os

def compile_module(source_path, output_path):
    """Compile source markdown to binary format."""
    # Read source
    with open(source_path, 'r') as f:
        content = f.read()
    
    # Process and compress content
    # ... your compilation logic ...
    
    # Write binary output
    with open(output_path, 'wb') as f:
        f.write(compressed_content)

if __name__ == '__main__':
    compile_module(
        'knowledge/my_module.source.md',
        'knowledge/my_module.tcz'
    )
```

### Step 3: Register Module

Add the module descriptor to your configuration:

```json
{
  "knowledge_modules": [
    {
      "id": "my_module",
      "type": "my.domain",
      "semantic_hints": "keywords for intent detection",
      "file_path": "knowledge/my_module.tcz"
    }
  ]
}
```

### Step 4: Test Module

```bash
# Compile the module
python3 tools/compile_my_module.py

# Test via API
curl -X POST http://localhost:8080/v1/knowledge/search \
  -H "Content-Type: application/json" \
  -d '{"query": "your test query", "limit": 5}'
```

---

## API Endpoints

### Search Knowledge

```http
POST /v1/knowledge/search
Content-Type: application/json

{
  "query": "What is machine learning?",
  "limit": 10
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "chunk_001",
      "score": 0.89,
      "text": "Machine learning is a subset of AI..."
    }
  ]
}
```

### Import Knowledge

```http
POST /v1/knowledge/import
Content-Type: application/json

{
  "text": "Your knowledge text here..."
}
```

**Response:**
```json
{
  "chunks_imported": 5,
  "status": "ok"
}
```

---

## Best Practices

1. **Keep modules small**: Aim for < 10 KB per module
2. **Use clear sections**: Organize content with headers
3. **Add semantic hints**: Help the system detect relevant intents
4. **Test thoroughly**: Verify module works with various queries
5. **Document safety boundaries**: Clearly state limitations

---

## Troubleshooting

### Module not loading
- Check file path in configuration
- Verify file is below 50 MiB budget
- Check file permissions

### Poor search results
- Add more semantic hints
- Improve content organization
- Check vector store configuration

### Memory issues
- Reduce module size
- Check for memory leaks
- Verify explicit drop after use
