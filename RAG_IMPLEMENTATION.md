# Complete RAG Agent Implementation with Pydantic AI & LangChain

## Overview
You now have a **fully-featured RAG (Retrieval-Augmented Generation) agent** powered by **Pydantic AI** and **LangChain** integrated across your Operarius system.

## What's Been Implemented

### 1. **Pydantic AI Models** (`rag_models.py`)
Type-safe structured models for the entire RAG system:
- `Document` - Knowledge base documents with metadata
- `DocumentChunk` - Text chunks with embeddings
- `Message` - Chat history with platform tracking
- `RetrievalQuery` - Structured search queries
- `RetrievalResult` - Search results with relevance scores
- `AgentResponse` - Type-safe agent responses with sources
- `FileUpload` - File metadata and processing status
- `SessionContext` - Multi-turn conversation context
- `RAGConfig` - System configuration

**Benefits:**
- Type safety and validation using Pydantic v2
- Auto-generated JSON schemas
- IDE autocompletion
- Runtime validation

### 2. **LangChain RAG Pipeline** (`rag_pipeline.py`)
Complete RAG system with:
- **Document Management**
  - Upload files (PDF, DOCX, TXT, MD, etc)
  - Automatic text chunking with configurable overlap
  - Full-text search on documents
  - Database persistence

- **Vector Store Integration (Chroma)**
  - HuggingFace embeddings (sentence-transformers)
  - Semantic similarity search
  - Persistent vector storage
  - Configurable retrieval parameters

- **RAG Chains**
  - RetrievalQA chains from LangChain
  - LLM integration with local llama.cpp
  - Source tracking and citation

- **Database Integration**
  - SQLite persistence for all data
  - Full-text search indexes on files and documents
  - Chat history tracking
  - Multi-user support

**Core Methods:**
```python
# Add documents
rag_pipeline.add_document(doc)

# Upload files  
file_upload = rag_pipeline.upload_file(file_path, user_id)

# Search
result = rag_pipeline.retrieve(retrieval_query)

# Query with RAG
response = rag_pipeline.query(query_text, session, user_id)

# Save messages
rag_pipeline.save_message(session_id, role, content)
```

### 3. **Pydantic AI Agent** (`pydantic_ai_agent.py`)
Main agent interface with:
- **Async/Sync Processing**
  - `process_message()` - async method
  - `process_message_sync()` - sync wrapper

- **Registered Tools**
  - `search_documents()` - Full-text + semantic search
  - `add_document()` - Add to knowledge base
  - `upload_file()` - Index files
  - `get_document_summary()` - Generate summaries

- **Features**
  - Automatic document retrieval
  - Context building from relevant docs
  - Confidence scoring
  - Source attribution
  - Multi-session support

### 4. **Hermes Gateway Integration** (`rag_integration.py`)
Bridge between Hermes and RAG system:
- Load config from `hermes/config.yaml`
- Process messages with RAG context
- Upload documents from any platform
- Search knowledge base
- Get KB statistics

**Functions:**
```python
# Get global integration instance
integration = get_rag_integration(config_path)

# Process message with RAG
result = integration.process_message(message, session_id, user_id, platform)

# Upload document
file_result = integration.upload_document(file_path, user_id, platform)

# Search
search_result = integration.search_documents(query, top_k, user_id)

# Stats
stats = integration.get_knowledge_base_stats(user_id)
```

### 5. **Tauri Backend Commands** (`commands.rs`)
New Rust commands for the web app:

```rust
// Upload document to RAG
upload_document(file_path, user_id, platform) -> String

// Search knowledge base
search_documents(query, limit) -> Vec<JSON>

// Query with RAG context
query_rag_agent(message, user_id) -> String

// Get all documents
get_knowledge_base(user_id) -> Vec<JSON>
```

### 6. **Database Schema Extensions**
New tables for RAG system:
- `files` - Uploaded files with metadata
- `knowledge_base` - Indexed documents
- `chat_history` - Cross-platform message history
- FTS indexes for `files` and `knowledge_base`
- Automatic triggers for full-text search

### 7. **Configuration** (`hermes/config.yaml`)
Updated with RAG settings:
```yaml
agent:
  framework: "pydantic-ai"
  use_langchain: true

rag:
  enabled: true
  top_k: 3
  similarity_threshold: 0.6
  chunk_size: 1024
  vector_store: "chroma"
  db_path: "/path/to/state.db"

embedding:
  model: "sentence-transformers/all-MiniLM-L6-v2"
  dimension: 384

memory:
  enabled: true
  max_turns: 10
```

### 8. **Dependencies Added**
To both `pyproject.toml` files:
- `pydantic-ai>=0.6.0` - Structured AI agent framework
- `langchain>=0.1.0` - LLM orchestration
- `langchain-community>=0.0.1` - Integrations
- `langchain-openai>=0.0.1` - LLM support
- `chroma>=0.4.0` - Vector store
- `sentence-transformers>=2.2.0` - Embeddings
- `aiosqlite>=0.19.0` - Async DB

Rust dependencies in `Cargo.toml`:
- `uuid` - Unique identifiers
- `chrono` - Timestamps

## How It Works End-to-End

### File Upload Flow
1. User sends file to Telegram or web app
2. Tauri command `upload_document()` is called
3. File is stored in SQLite `files` table
4. Content is extracted (PDF, DOCX, TXT support)
5. RAG pipeline chunks the text
6. Chunks are embedded with sentence-transformers
7. Embeddings stored in Chroma vector DB
8. Document indexed in `knowledge_base` table
9. FTS indexes updated automatically

### Query Flow
1. User sends message to Telegram or web app
2. Hermes gateway receives message
3. RAG integration retrieves relevant documents
4. LangChain RAG chain retrieves top-k chunks
5. Pydantic AI agent processes with context
6. LLM generates response with sources
7. Response saved to `chat_history`
8. Message history persisted for learning

### Cross-Platform Sync
- **Telegram** - Messages sync automatically
- **Web App** - Via Tauri commands
- **Discord/Slack** - Via Hermes gateway
- **Database** - Single source of truth
- **User Context** - Preserved across platforms

## Usage Examples

### In Python (Hermes Gateway)
```python
from rag_integration import get_rag_integration

# Initialize
rag = get_rag_integration()

# Process message with RAG
response = rag.process_message(
    "What's in my documents?",
    user_id="user123",
    platform="telegram"
)
print(response["content"])
print(response["sources"])

# Upload document
file_result = rag.upload_document(
    "/path/to/document.pdf",
    user_id="user123"
)

# Search
search_result = rag.search_documents(
    "query text",
    top_k=3,
    user_id="user123"
)
```

### In Rust (Tauri App)
```rust
// Upload document
let result = upload_document(
    app,
    pool,
    "/path/to/file.pdf".to_string(),
    Some("user123".to_string()),
    Some("app".to_string())
).await?;

// Query with RAG
let response = query_rag_agent(
    app,
    pool,
    "What's in my files?".to_string(),
    Some("user123".to_string())
).await?;

// Search documents
let results = search_documents(
    app,
    pool,
    "search query".to_string(),
    Some(5)
).await?;
```

### In Telegram Bot (Python)
```python
# Message received
message_text = "Can you summarize my documents?"

# Process with RAG automatically
response = await rag_agent.process_message(
    message_text,
    user_id=user_id,
    platform="telegram"
)

# Send response with sources
await bot.send_message(
    chat_id,
    f"{response.content}\n\nSources: {response.sources}"
)
```

## Key Features

✅ **Type-Safe** - Pydantic models with validation
✅ **Scalable** - LangChain chains for complex orchestration
✅ **Fast** - Semantic search with vector embeddings
✅ **Persistent** - SQLite storage with FTS indexes
✅ **Cross-Platform** - Works on Telegram, Web, Discord, etc
✅ **Self-Learning** - Conversation history for improvement
✅ **Configurable** - Via `hermes/config.yaml`
✅ **Traceable** - Source attribution and citations

## Next Steps

1. **Install Dependencies**
   ```bash
   python3 -m pip install pydantic-ai langchain langchain-community sentence-transformers chromadb
   ```

2. **Restart Services**
   ```bash
   pkill -9 -f "hermes" && sleep 3
   pkill -9 -f "llama"
   # Supervisor will restart automatically
   ```

3. **Test RAG Queries**
   - Send a file to Telegram
   - Ask a question about it
   - Agent will retrieve and respond with sources

4. **Monitor** 
   - Check `state.db` for documents
   - Review `chroma_db/` for embeddings
   - Inspect `chat_history` table for interactions

## Architecture Diagram

```
User Input (Telegram/Web/Discord)
    ↓
[Hermes Gateway / Tauri App]
    ↓
[RAG Integration Layer]
    ├─ Document Upload → SQLite + Chroma
    ├─ Message Search → FTS + Vector Search
    └─ Query Processing
    ↓
[Pydantic AI Agent]
    ├─ Tool Registration (search, add, upload)
    ├─ Context Building
    └─ Response Generation
    ↓
[LangChain RAG Chain]
    ├─ LLM (llama.cpp)
    ├─ Vector Store (Chroma)
    └─ Document Retrieval
    ↓
[Database] (SQLite)
    ├─ knowledge_base (FTS)
    ├─ files
    ├─ chat_history
    └─ embeddings (Chroma)
    ↓
Response with Sources
```

## Configuration Best Practices

**For Speed:**
```yaml
rag:
  top_k: 2
  chunk_size: 512
  similarity_threshold: 0.7
```

**For Accuracy:**
```yaml
rag:
  top_k: 5
  chunk_size: 1024
  similarity_threshold: 0.5
```

**For Development:**
```yaml
rag:
  enabled: true
  storage_type: "sqlite"
  vector_store: "chroma"  # In-memory by default
```

## Troubleshooting

- **"RAG agent not initialized"** - Check dependencies are installed
- **"No embeddings generated"** - Ensure sentence-transformers is installed
- **"Vector DB error"** - Check `chroma_db/` directory permissions
- **"File upload fails"** - Ensure file extension is supported
- **"Slow queries"** - Reduce `top_k` or increase `similarity_threshold`

---

Your Operarius system now has **enterprise-grade RAG capabilities** with full integration across all platforms! 🚀
