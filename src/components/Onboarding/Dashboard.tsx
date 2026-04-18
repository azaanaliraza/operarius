import React from 'react';
import { Plus, Search, Settings, Grid, MessageSquare, BookOpen, Layers } from 'lucide-react';

const Dashboard: React.FC = () => {
  return (
    <div className="min-h-screen bg-[#F8F9FA] flex font-sans">
      {/* Sidebar */}
      <aside className="w-20 bg-white border-r border-gray-100 flex flex-col items-center py-8 gap-8">
        <div className="w-12 h-12 bg-black rounded-2xl flex items-center justify-center text-white text-2xl shadow-lg mb-4">🐆</div>
        <nav className="flex flex-col gap-6">
          <button className="p-3 bg-black text-white rounded-2xl shadow-lg"><Grid className="w-6 h-6" /></button>
          <button className="p-3 text-gray-400 hover:text-black hover:bg-gray-50 rounded-2xl transition-all"><MessageSquare className="w-6 h-6" /></button>
          <button className="p-3 text-gray-400 hover:text-black hover:bg-gray-50 rounded-2xl transition-all"><BookOpen className="w-6 h-6" /></button>
          <button className="p-3 text-gray-400 hover:text-black hover:bg-gray-50 rounded-2xl transition-all"><Layers className="w-6 h-6" /></button>
        </nav>
        <div className="mt-auto">
          <button className="p-3 text-gray-400 hover:text-black hover:bg-gray-50 rounded-2xl transition-all"><Settings className="w-6 h-6" /></button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 p-10">
        <header className="flex justify-between items-center mb-12">
          <div>
            <h1 className="text-3xl font-bold text-[#1C1C1E] mb-1">Good evening, Azaan.</h1>
            <p className="text-[#6B7280] font-medium">Your agents running locally on Hermes (Llama 3.1).</p>
          </div>
          <div className="flex items-center gap-4">
            <div className="relative">
              <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input 
                type="text" 
                placeholder="Search resources..." 
                className="pl-12 pr-6 py-3 bg-white border border-gray-100 rounded-2xl w-64 focus:outline-none focus:border-black transition-all"
              />
            </div>
            <button className="bg-black text-white px-6 py-3 rounded-2xl font-bold flex items-center gap-2 hover:bg-gray-900 transition-all shadow-xl">
              <Plus className="w-5 h-5" /> Create Agent
            </button>
          </div>
        </header>

        {/* Dashboard Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
          <div className="bg-white p-8 rounded-[2.5rem] shadow-sm border border-gray-50 hover:shadow-xl transition-all group flex flex-col cursor-pointer">
            <div className="w-12 h-12 bg-blue-50 text-blue-600 rounded-xl flex items-center justify-center mb-6 font-bold group-hover:bg-blue-600 group-hover:text-white transition-all">01</div>
            <h3 className="text-xl font-bold mb-2">Hermes Core</h3>
            <p className="text-[#6B7280] text-sm leading-relaxed mb-6">The main orchestrator agent. Handles RAG, memory loops, and tool calling.</p>
            <div className="mt-auto flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
              <span className="text-[10px] font-bold text-[#9CA3AF] uppercase">Active & Learning</span>
            </div>
          </div>

          <div className="bg-white p-8 rounded-[2.5rem] shadow-sm border border-gray-50 border-dashed flex flex-col items-center justify-center text-center group cursor-pointer hover:border-black transition-all">
            <div className="w-16 h-16 bg-gray-50 rounded-full flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
              <Plus className="w-8 h-8 text-gray-300 group-hover:text-black" />
            </div>
            <h3 className="text-lg font-bold">New Skill</h3>
            <p className="text-[#6B7280] text-xs max-w-[160px] mx-auto">Teach Hermes a new capability or routine.</p>
          </div>
        </div>
      </main>
    </div>
  );
};

export default Dashboard;
