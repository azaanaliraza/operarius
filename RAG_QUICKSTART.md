# RAG Agent - Quick Start Guide

## Installation

```bash
# Install Python dependencies
python3 -m pip install --upgrade \
  pydantic-ai \
  langchain \
  langchain-community \
  sentence-transformers \
  chromadb \
  aiosqlite

# Build Rust dependencies (if needed)
cd src-tauri && cargo build
```

## Configuration Files

All critical files are in place:
- ✅ `rag_models.py` - Type definitions
- ✅ `rag_pipeline.py` - RAG engine
- ✅ `pydantic_ai_agent.py` - Agent interface
- ✅ `rag_integration.py` - Hermes bridge
- ✅ `hermes/config.yaml` - System config
- ✅ Database schema - SQLite tables created

## Start the System

```bash
# Kill existing processes
pkill -9 -f "hermes"
pkill -9 -f "llama"

# Wait for supervisor to restart (auto)
sleep 5

# Verify status
ps aux | grep -E "hermes|llama" | grep -v grep
```

## Test RAG Workflow

### 1. Upload a Document (Telegram)
```
Send: /upload_document path/to/document.pdf
Expected: "Document indexed: file_id"
```

### 2. Query with RAG (Web App)
```
Click: Upload button → select file
Result: Document stored in knowledge_base
```

### 3. Ask Question (Telegram)
```
Send: "What's in my documents?"
Expected: Response with sources cited
```

## Common Commands

### Upload Document
```python
# Python/Hermes
rag = get_rag_integration()
result = rag.upload_document("/path/file.pdf", user_id="123", platform="telegram")
```

```rust
// Rust/Tauri
upload_document(app, pool, path, user_id, platform)
```

### Search Knowledge Base
```python
result = rag.search_documents("search query", top_k=3, user_id="123")
# Returns: {"results": [...], "count": N, "total_score": 0.8}
```

### Process Message with RAG
```python
response = rag.process_message(
    "Question about docs",
    session_id="sess_123",
    user_id="user_123",
    platform="telegram"
)
# Returns: {"content": "...", "confidence": 0.95, "sources": [...]}
```

## File Support
- ✅ PDF (.pdf)
- ✅ Word (.docx, .doc)
- ✅ Text (.txt)
- ✅ Markdown (.md)
- ✅ HTML (.html)
- ✅ JSON (.json)

## Monitoring

### Check Database
```bash
sqlite3 ~/Documents/Operarius/state.db
> SELECT COUNT(*) FROM knowledge_base;
> SELECT * FROM files WHERE status='indexed';
> SELECT * FROM chat_history LIMIT 5;
```

### Monitor Logs
```bash
# Hermes logs
tail -f ~/.hermes/hermes.log

# Llama inference
tail -f ~/Documents/Operarius/llama.log
```

### Vector Store Status
```bash
ls -la ~/Documents/Operarius/chroma_db/
```

## Performance Tuning

### For Speed (Real-time)
```yaml
rag:
  top_k: 2
  chunk_size: 512
  similarity_threshold: 0.75
```

### For Accuracy (Research)
```yaml
rag:
  top_k: 5
  chunk_size: 1024
  similarity_threshold: 0.5
```

### Memory Optimization
```yaml
rag:
  cache_documents: false
  vector_store: "chroma"  # Disk-backed
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "RAG agent not initialized" | Install pydantic-ai and langchain |
| "No embeddings" | Install sentence-transformers |
| "Vector store error" | Check `chroma_db/` permissions |
| "Slow queries" | Increase `similarity_threshold` |
| "Out of memory" | Reduce `chunk_size` or `top_k` |
| "Files not indexing" | Check file format is supported |

## Architecture

```
User (Telegram/Web/Discord)
    ↓
Hermes Gateway / Tauri App
    ↓
RAG Integration (rag_integration.py)
    ↓
Pydantic AI Agent (pydantic_ai_agent.py)
    ↓
LangChain RAG (rag_pipeline.py)
    ├─ Chroma Vector Store
    ├─ Sentence-transformers Embeddings
    └─ LLM (llama.cpp)
    ↓
SQLite Database
    ├─ knowledge_base (FTS)
    ├─ files
    └─ chat_history
```

## Next Steps

1. **Deploy**: Restart services and test
2. **Upload**: Add your documents
3. **Query**: Ask questions about them
4. **Monitor**: Check performance
5. **Tune**: Adjust config for your use case

## Support

Check [RAG_IMPLEMENTATION.md](RAG_IMPLEMENTATION.md) for detailed documentation.

---

**Your RAG agent is ready to enhance Operarius with document intelligence!** 🚀
