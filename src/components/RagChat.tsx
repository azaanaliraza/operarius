import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Send, Bot, User, Cpu, Sparkles, Terminal } from 'lucide-react';

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

const RagChat: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, isTyping]);

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

    try {
      const response = await invoke<string>('chat_with_rag_agent', { message: currentInput });
      const assistantMsg: Message = { 
        role: 'assistant', 
        content: response,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      };
      setMessages(prev => [...prev, assistantMsg]);
    } catch (err) {
      console.error(err);
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Something went wrong: ${String(err)}. The AI engine may still be loading — try again in a moment.`,
        timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      }]);
    } finally {
      setIsTyping(false);
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
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-8 space-y-10 no-scrollbar pb-32">
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
                  <div className={`p-5 rounded-[2rem] leading-relaxed text-sm font-medium shadow-sm transition-all
                    ${msg.role === 'user' 
                      ? 'bg-black text-white rounded-tr-none' 
                      : 'bg-white border border-[#E5E5E7] text-[#1C1C1E] rounded-tl-none hover:border-black/10'}`}>
                    {msg.content}
                  </div>
                  <div className={`text-[8px] font-black opacity-30 uppercase tracking-widest ${msg.role === 'user' ? 'text-right' : 'text-left'}`}>
                    {msg.timestamp}
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
               </div>
             </div>
          </div>
        )}
      </div>

      {/* INPUT COMMANDER */}
      <div className="p-8 absolute bottom-0 left-0 right-0 bg-gradient-to-t from-[#FAFAFA] via-[#FAFAFA] to-transparent">
        <div className="max-w-4xl mx-auto relative group">
          <div className="absolute inset-0 bg-black/5 rounded-[2.5rem] blur-xl opacity-0 group-focus-within:opacity-100 transition-all duration-500"></div>
          <div className="relative bg-white border border-[#E5E5E7] rounded-[2.5rem] p-1.5 pl-8 flex items-center shadow-xl group-focus-within:border-black/20 transition-all">
            <input
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyPress={e => e.key === 'Enter' && sendMessage()}
              placeholder="Query local knowledge vault..."
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
        </div>
      </div>
    </div>
  );
};

export default RagChat;
