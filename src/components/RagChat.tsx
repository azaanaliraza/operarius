import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { Send, Bot, User, Cpu, Sparkles, Terminal, Paperclip, Globe, Brain, Database } from 'lucide-react';

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
  platform?: string;
  eventType?: string | null;
  eventPayload?: string | null;
}

interface ChatHistoryMessage {
  role: string;
  content: string;
  timestamp: string;
  platform: string;
  chat_id?: string | null;
  event_type?: string | null;
  event_payload?: string | null;
}

interface UploadEventPayload {
  filename?: string;
  size_bytes?: number;
  status?: string;
  source?: string;
}

interface IndexedFile {
  filename: string;
  platform?: string | null;
  uploaded_at: number;
}

interface FileMetadata {
  filename: string;
  size_bytes: number;
}

interface UploadPopup {
  filename: string;
  sizeLabel: string;
  status: 'indexing' | 'indexed' | 'error';
  message?: string;
}

const STATUS_SEQUENCE = [
  'Mulling over your request...',
  'Loading memory and file context...',
  'Scanning knowledge sources...',
  'Assembling response plan...',
  'Generating final response...',
];

const PHASE_TO_STATUS: Record<string, string> = {
  mulling: 'Mulling over your request...',
  'memory-scan': 'Loading memory and file context...',
  'web-search': 'Searching the web for fresh references...',
  'local-scan': 'Scanning local knowledge base...',
  'deep-retrieval': 'Running deep retrieval across indexed chunks...',
  'file-context': 'Indexing selected file context for answer...',
  generation: 'Generating final response...',
};

const RagChat: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [indexedFiles, setIndexedFiles] = useState<IndexedFile[]>([]);
  const [activeFile, setActiveFile] = useState('');
  const [stickToBottom, setStickToBottom] = useState(true);
  const [uploadPopup, setUploadPopup] = useState<UploadPopup | null>(null);
  const [webSearchEnabled, setWebSearchEnabled] = useState(false);
  const [deepThinkingEnabled, setDeepThinkingEnabled] = useState(false);
  const [memoryUseEnabled, setMemoryUseEnabled] = useState(true);
  const [generationStatus, setGenerationStatus] = useState('Mulling over your request...');
  const scrollRef = useRef<HTMLDivElement>(null);

  const formatTimestamp = (raw: string) => {
    if (!raw) {
      return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    const parsed = new Date(raw);
    if (!Number.isNaN(parsed.getTime())) {
      return parsed.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    return raw;
  };

  const formatReadableContent = (text: string) => {
    return text
      .replace(/\r\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  };

  const parseUploadEvent = (msg: Message): UploadEventPayload | null => {
    if (msg.eventType !== 'file_upload') {
      return null;
    }
    if (!msg.eventPayload) {
      return null;
    }
    try {
      return JSON.parse(msg.eventPayload) as UploadEventPayload;
    } catch {
      return null;
    }
  };

  const loadHistory = async () => {
    try {
      const history = await invoke<ChatHistoryMessage[]>('get_chat_history');
      setMessages(
        history.map((item) => ({
          role: item.role === 'assistant' ? 'assistant' : 'user',
          content: item.content,
          timestamp: formatTimestamp(item.timestamp),
          platform: item.platform,
          eventType: item.event_type,
          eventPayload: item.event_payload,
        }))
      );
    } catch (err) {
      console.error('Failed to load chat history', err);
    }
  };

  const loadIndexedFiles = async () => {
    try {
      const files = await invoke<IndexedFile[]>('get_indexed_files');
      setIndexedFiles(files);
      if (!activeFile && files.length > 0) {
        setActiveFile(files[0].filename);
      }
    } catch (err) {
      console.error('Failed to load indexed files', err);
    }
  };

  useEffect(() => {
    if (scrollRef.current && stickToBottom) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, isTyping, stickToBottom]);

  useEffect(() => {
    loadIndexedFiles();
  }, []);

  useEffect(() => {
    let isMounted = true;

    const initializeHistory = async () => {
      if (!isMounted) return;
      await loadHistory();
      await loadIndexedFiles();
    };

    initializeHistory();

    const unlistenPromise = listen('chat-history-updated', async () => {
      if (!isMounted) return;
      await loadHistory();
      await loadIndexedFiles();
    });

    return () => {
      isMounted = false;
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const unlistenPromise = listen<Record<string, unknown>>('agent-log', (event) => {
      const payload = event.payload || {};
      if (String(payload.scope || '') !== 'chat') {
        return;
      }

      const details = (payload.details as Record<string, unknown> | undefined) || undefined;
      const phase = details && typeof details.phase === 'string' ? details.phase : '';
      if (phase && PHASE_TO_STATUS[phase]) {
        setGenerationStatus(PHASE_TO_STATUS[phase]);
        return;
      }

      const message = String(payload.message || '').trim();
      if (message) {
        setGenerationStatus(message);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!isTyping) {
      setGenerationStatus('Ready');
      return;
    }

    const phases: string[] = [
      STATUS_SEQUENCE[0],
      STATUS_SEQUENCE[1],
      webSearchEnabled ? 'Searching the web for fresh references...' : 'Scanning local knowledge base...',
      deepThinkingEnabled ? 'Running deep retrieval across indexed chunks...' : STATUS_SEQUENCE[3],
      STATUS_SEQUENCE[4],
    ];

    let index = 0;
    setGenerationStatus(phases[index]);

    const interval = setInterval(() => {
      if (index >= phases.length - 1) {
        clearInterval(interval);
        return;
      }

      index += 1;
      setGenerationStatus(phases[index]);
    }, 2600);

    return () => clearInterval(interval);
  }, [isTyping, webSearchEnabled, deepThinkingEnabled]);

  const onChatScroll = () => {
    if (!scrollRef.current) return;
    const el = scrollRef.current;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 90;
    setStickToBottom(nearBottom);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const sendMessage = async () => {
    if (!input.trim() || isTyping) return;
    
    const userMsg: Message = { 
      role: 'user', 
      content: input,
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    };
    
    setMessages(prev => [...prev, userMsg]);
    const currentInput = input;
    setInput('');
    setIsTyping(true);
    setUploadPopup(null);
    setGenerationStatus('Mulling over your request...');

    try {
      const response = await invoke<string>('chat_with_rag_agent', {
        message: currentInput,
        targetFile: activeFile || undefined,
        webSearch: webSearchEnabled,
        deepThinking: deepThinkingEnabled,
        memoryUse: memoryUseEnabled,
      });

      if (memoryUseEnabled) {
        await loadHistory();
      } else {
        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: response,
            timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
            platform: 'app',
          },
        ]);
      }
    } catch (err) {
      console.error(err);
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Something went wrong: ${String(err)}. The AI engine may still be loading — try again in a moment.`,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      }]);
    } finally {
      setIsTyping(false);
      setGenerationStatus('Ready');
    }
  };

  const uploadFile = async () => {
    if (isUploading || isTyping) return;

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: 'Select file to index for RAG',
      });

      if (!selected || Array.isArray(selected)) return;

      setIsUploading(true);
      const filePath = String(selected);
      const metadata = await invoke<FileMetadata>('get_file_metadata', { filePath });
      setUploadPopup({
        filename: metadata.filename,
        sizeLabel: formatFileSize(metadata.size_bytes),
        status: 'indexing',
      });

      await invoke<string>('upload_document', {
        filePath,
        userId: 'local-user',
        platform: 'app',
        memoryUse: memoryUseEnabled,
      });
      const uploadedName = filePath.split('/').pop() || filePath;
      setActiveFile(uploadedName);
      await loadIndexedFiles();
      setUploadPopup({
        filename: metadata.filename,
        sizeLabel: formatFileSize(metadata.size_bytes),
        status: 'indexed',
        message: 'Fully indexed and ready for file-specific Q&A',
      });

      if (memoryUseEnabled) {
        await loadHistory();
      }
    } catch (err) {
      console.error('File upload failed', err);
      setUploadPopup((current) => ({
        filename: current?.filename || 'Upload failed',
        sizeLabel: current?.sizeLabel || '--',
        status: 'error',
        message: String(err),
      }));
      setMessages((prev) => [
        ...prev,
        {
          role: 'assistant',
          content: `Failed to open or index file: ${String(err)}`,
          timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
          platform: 'app',
        },
      ]);
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-[#FAFAFA] overflow-hidden">
      {/* KNOWLEDGE STATUS BAR */}
      <div className="h-12 px-6 flex items-center justify-between border-b border-[#E5E5E7] bg-white text-[10px] font-black uppercase tracking-widest text-black/40">
        <div className="flex items-center gap-3">
          <div className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse shadow-[0_0_8px_rgba(16,185,129,0.5)]"></div>
          <span>Local Index: [Active]</span>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1.5">
             <Cpu className="w-3 h-3" />
             <span>NPU Accelerated</span>
          </div>
          <div className="h-3 w-[1px] bg-gray-200"></div>
          <span>Memory Mode</span>
        </div>
      </div>

      {/* CHAT ARENA */}
      <div ref={scrollRef} onScroll={onChatScroll} className="flex-1 overflow-y-auto p-8 space-y-10 no-scrollbar pb-36">
        {indexedFiles.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            <button
              onClick={() => setActiveFile('')}
              className={`px-3 py-1 rounded-full text-[10px] font-black tracking-wider border transition-all ${
                activeFile ? 'bg-white border-gray-200 text-gray-500' : 'bg-black text-white border-black'
              }`}
            >
              ALL FILES
            </button>
            {indexedFiles.slice(0, 30).map((file) => (
              <button
                key={`${file.filename}-${file.uploaded_at}`}
                onClick={() => setActiveFile(file.filename)}
                className={`px-3 py-1 rounded-full text-[10px] font-black tracking-wider border transition-all ${
                  activeFile === file.filename
                    ? 'bg-black text-white border-black'
                    : 'bg-white border-gray-200 text-gray-500 hover:text-black'
                }`}
                title={`Indexed via ${file.platform || 'app'}`}
              >
                {file.filename}
              </button>
            ))}
          </div>
        )}

        {messages.length === 0 && (
          <div className="h-full flex flex-col items-center justify-center opacity-20">
             <div className="w-16 h-16 bg-black rounded-[2rem] flex items-center justify-center mb-6">
                <Sparkles className="w-8 h-8 text-white" />
             </div>
             <p className="text-xs font-black uppercase tracking-[0.4em]">Initialize Neural Recall</p>
          </div>
        )}

        {messages.map((msg, i) => (
          <div key={i} className={`flex w-full animate-in slide-in-from-bottom-2 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
            <div className={`flex gap-4 max-w-[85%] lg:max-w-[70%] ${msg.role === 'user' ? 'flex-row-reverse' : 'flex-row'}`}>
               <div className={`w-8 h-8 rounded-xl flex items-center justify-center shrink-0 shadow-sm border
                 ${msg.role === 'user' ? 'bg-black border-black border-b-2' : 'bg-white border-gray-100'}`}>
                  {msg.role === 'user' ? <User className="w-4 h-4 text-white" /> : <Bot className="w-4 h-4 text-black" />}
               </div>
               
               <div className="space-y-2">
                  {parseUploadEvent(msg) ? (
                    <div className="p-4 rounded-[1.6rem] leading-relaxed text-sm font-medium shadow-sm transition-all bg-white border border-[#E5E5E7] text-[#1C1C1E] rounded-tl-none hover:border-black/10 min-w-[260px]">
                      <div className="flex items-center justify-between gap-3">
                        <div className="min-w-0">
                          <div className="text-[11px] font-black uppercase tracking-[0.15em] text-[#1C1C1E] truncate">
                            {(parseUploadEvent(msg)?.filename || msg.content.replace('Uploaded file:', '').trim())}
                          </div>
                          <div className="text-[10px] font-bold uppercase tracking-widest text-black/40 mt-1">
                            {parseUploadEvent(msg)?.size_bytes ? formatFileSize(parseUploadEvent(msg)?.size_bytes || 0) : 'FILE'}
                          </div>
                        </div>
                        <div className="text-[10px] font-black uppercase tracking-widest px-3 py-1 rounded-full bg-emerald-100 text-emerald-700">
                          {parseUploadEvent(msg)?.status || 'indexed'}
                        </div>
                      </div>
                      <div className="mt-2 text-[11px] font-medium text-black/70">
                        Uploaded and indexed for retrieval-aware answers.
                      </div>
                    </div>
                  ) : (
                    <div className={`p-5 rounded-[2rem] leading-relaxed text-sm font-medium shadow-sm transition-all
                      ${msg.role === 'user' 
                        ? 'bg-black text-white rounded-tr-none' 
                        : 'bg-white border border-[#E5E5E7] text-[#1C1C1E] rounded-tl-none hover:border-black/10'} whitespace-pre-wrap break-words`}>
                      {formatReadableContent(msg.content)}
                    </div>
                  )}
                  <div className={`text-[8px] font-black opacity-30 uppercase tracking-widest ${msg.role === 'user' ? 'text-right' : 'text-left'}`}>
                    {msg.platform === 'telegram' ? 'Telegram • ' : 'App • '}{msg.timestamp}
                  </div>
               </div>
            </div>
          </div>
        ))}

        {isTyping && (
          <div className="flex justify-start animate-in fade-in duration-300">
             <div className="flex gap-4">
               <div className="w-8 h-8 rounded-xl bg-white border border-gray-100 flex items-center justify-center shadow-sm">
                 <Terminal className="w-4 h-4 text-black animate-pulse" />
               </div>
               <div className="bg-white border border-[#E5E5E7] p-4 px-6 rounded-[2rem] rounded-tl-none flex gap-1 items-center shadow-sm">
                  <div className="w-1.5 h-1.5 bg-black/20 rounded-full animate-bounce [animation-delay:-0.3s]"></div>
                  <div className="w-1.5 h-1.5 bg-black/20 rounded-full animate-bounce [animation-delay:-0.15s]"></div>
                  <div className="w-1.5 h-1.5 bg-black/20 rounded-full animate-bounce"></div>
                  <div className="ml-3 text-[10px] font-black uppercase tracking-widest text-black/50">
                    {generationStatus}
                  </div>
               </div>
             </div>
          </div>
        )}
      </div>

      {/* INPUT COMMANDER */}
      <div className="p-8 absolute bottom-0 left-0 right-0 z-20 bg-gradient-to-t from-[#FAFAFA] via-[#FAFAFA] to-transparent">
        <div className="max-w-4xl mx-auto relative group">
          {uploadPopup && (
            <div className="mb-3 px-5 py-3 bg-white border border-[#E5E5E7] rounded-[1.4rem] shadow-lg">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <div className="text-[11px] font-black uppercase tracking-[0.15em] text-[#1C1C1E] truncate">
                    {uploadPopup.filename}
                  </div>
                  <div className="text-[10px] font-bold uppercase tracking-widest text-black/40 mt-1">
                    {uploadPopup.sizeLabel}
                  </div>
                </div>
                <div
                  className={`text-[10px] font-black uppercase tracking-widest px-3 py-1 rounded-full ${
                    uploadPopup.status === 'indexing'
                      ? 'bg-amber-100 text-amber-700'
                      : uploadPopup.status === 'indexed'
                      ? 'bg-emerald-100 text-emerald-700'
                      : 'bg-rose-100 text-rose-700'
                  }`}
                >
                  {uploadPopup.status}
                </div>
              </div>
              {uploadPopup.message && (
                <div className="mt-2 text-[11px] font-medium text-black/70">{uploadPopup.message}</div>
              )}
            </div>
          )}

          <div className="absolute inset-0 bg-black/5 rounded-[2.5rem] blur-xl opacity-0 pointer-events-none group-focus-within:opacity-100 transition-all duration-500"></div>
          <div className="relative z-10 bg-white border border-[#E5E5E7] rounded-[2.5rem] p-1.5 pl-8 flex items-center shadow-xl group-focus-within:border-black/20 transition-all">
            <button
              onClick={uploadFile}
              disabled={isUploading || isTyping}
              className="w-10 h-10 mr-2 rounded-full flex items-center justify-center text-black/70 hover:bg-black/5 disabled:opacity-40 transition-all"
              title="Upload file/image for RAG indexing"
            >
              <Paperclip className="w-4 h-4" />
            </button>
            <input
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyPress={e => e.key === 'Enter' && sendMessage()}
              placeholder={isUploading ? 'Indexing file...' : 'Query local knowledge vault...'}
              className="flex-1 bg-transparent py-4 text-sm font-medium placeholder:text-gray-300 outline-none"
            />
            <button 
              onClick={sendMessage}
              disabled={!input.trim() || isTyping}
              className={`w-12 h-12 rounded-full flex items-center justify-center transition-all
                ${input.trim() && !isTyping ? 'bg-black text-white hover:scale-105 active:scale-95' : 'bg-gray-50 text-gray-200'}`}
            >
              <Send className="w-4 h-4" />
            </button>
          </div>

          <div className="mt-3 px-1 flex flex-wrap items-center gap-2 relative z-10 pointer-events-auto">
            <button
              onClick={() => setWebSearchEnabled((v) => !v)}
              className={`h-8 px-3 rounded-full border text-[10px] font-black uppercase tracking-widest transition-all inline-flex items-center gap-1.5 ${
                webSearchEnabled
                  ? 'bg-emerald-50 border-emerald-200 text-emerald-600 shadow-sm'
                  : 'bg-white text-black/60 border-[#E5E5E7] hover:text-black hover:border-black/20'
              }`}
              title="Enable external web search for this chat"
            >
              <Globe className="w-3.5 h-3.5" />
              Web Search
            </button>

            <button
              onClick={() => setDeepThinkingEnabled((v) => !v)}
              className={`h-8 px-3 rounded-full border text-[10px] font-black uppercase tracking-widest transition-all inline-flex items-center gap-1.5 ${
                deepThinkingEnabled
                  ? 'bg-emerald-50 border-emerald-200 text-emerald-600 shadow-sm'
                  : 'bg-white text-black/60 border-[#E5E5E7] hover:text-black hover:border-black/20'
              }`}
              title="Enable deeper and longer reasoning response"
            >
              <Brain className="w-3.5 h-3.5" />
              Deep Thinking
            </button>

            <button
              onClick={() => setMemoryUseEnabled((v) => !v)}
              className={`h-8 px-3 rounded-full border text-[10px] font-black uppercase tracking-widest transition-all inline-flex items-center gap-1.5 ${
                memoryUseEnabled
                  ? 'bg-emerald-50 border-emerald-200 text-emerald-600 shadow-sm'
                  : 'bg-white text-black/60 border-[#E5E5E7] hover:text-black hover:border-black/20'
              }`}
              title="Use conversation and uploaded file memory"
            >
              <Database className="w-3.5 h-3.5" />
              Memory Use
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default RagChat;
