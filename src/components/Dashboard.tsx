import React, { useState } from 'react';
import { MessageCircle, BarChart3, Clock, Plus, Settings, LogOut, AppWindow, ArrowRight, Zap, Target, BookOpen } from 'lucide-react';
import ConnectPanel from './ConnectPanel';
import AgentBuilder from './AgentBuilder';

const Dashboard: React.FC = () => {
  const [showConnect, setShowConnect] = useState(false);
  const [showBuilder, setShowBuilder] = useState(false);

  return (
    <div className="flex h-screen bg-[#F8F9FA] font-sans selection:bg-black selection:text-white overflow-hidden">
      {/* Sidebar - Precision Miniaturized */}
      <aside className="w-20 md:w-64 bg-white border-r border-gray-100 flex flex-col shadow-sm transition-all duration-500 relative z-30">
        <div className="p-4 md:p-6 flex items-center gap-3 border-b border-gray-50 h-20">
          <div className="w-9 h-9 bg-black rounded-xl flex items-center justify-center text-white text-2xl shadow-lg flex-shrink-0">🐆</div>
          <div className="hidden md:block">
            <div className="font-bold text-sm tracking-tighter text-[#1C1C1E] uppercase">Operarius</div>
            <div className="text-[7px] font-bold text-emerald-600 uppercase tracking-widest mt-0.5">Local • v0.1</div>
          </div>
        </div>

        <nav className="p-4 flex-1 overflow-y-auto no-scrollbar">
          <div className="hidden md:block text-[8px] font-bold text-[#9CA3AF] uppercase tracking-[0.2em] mb-3 ml-2">Control</div>
          <div className="space-y-1">
            <NavItem icon={<MessageCircle className="w-4 h-4" />} label="Chat" active={!showBuilder} onClick={() => setShowBuilder(false)} />
            <NavItem icon={<Target className="w-4 h-4" />} label="Automation" active={showBuilder} onClick={() => setShowBuilder(true)} />
            <NavItem icon={<BarChart3 className="w-4 h-4" />} label="Metrics" />
            <NavItem icon={<Clock className="w-4 h-4" />} label="Threads" />
          </div>

          <div className="hidden md:block text-[8px] font-bold text-[#9CA3AF] uppercase tracking-[0.2em] mb-3 ml-2 mt-8">Deployment</div>
          <div className="space-y-1 hidden md:block">
            <AgentItem label="Morning Intel" color="bg-emerald-500" />
            <AgentItem label="Market Scout" color="bg-blue-500" />
            <AgentItem label="Email Filter" color="bg-amber-500" inactive />
          </div>
        </nav>

        {/* Connect Button */}
        <div className="p-4 border-t border-gray-50">
          <button 
            onClick={() => setShowConnect(true)}
            className="w-full bg-black text-white py-3 rounded-xl font-bold shadow-xl flex items-center justify-center gap-2 hover:bg-[#2D2D2E] active:scale-[0.98] transition-all duration-300 text-xs"
          >
            <AppWindow className="w-4 h-4" />
            <span className="hidden md:block">Connect</span>
          </button>
        </div>

        <div className="p-4 flex items-center justify-center md:justify-between px-6 h-16 opacity-40">
           <Settings className="w-4 h-4 hover:text-black cursor-pointer" />
           <LogOut className="w-4 h-4 hover:text-red-500 cursor-pointer" />
        </div>
      </aside>

      {/* Main Content - No Scroll, Perfect Fit */}
      <main className="flex-1 p-6 md:p-10 flex flex-col h-full bg-[#F8F9FA]">
        <header className="flex flex-col md:flex-row md:justify-between md:items-start mb-8 gap-4">
          <div className="max-w-xl">
            <h1 className="text-fluid-xl font-bold tracking-tight text-[#1C1C1E] mb-2 leading-none">Automate Anything.</h1>
            <p className="text-xs md:text-sm text-[#6B7280] font-medium opacity-70">
              Describe a workflow. Operarius will deploy as a local agent instantly.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button className="bg-white border border-gray-100 px-4 py-2 rounded-xl font-bold shadow-sm hover:border-black transition-all text-[10px] uppercase tracking-widest">
              Library
            </button>
            <button 
              onClick={() => setShowBuilder(true)}
              className="bg-black text-white px-5 py-2.5 rounded-xl font-bold shadow-lg flex items-center gap-2 hover:bg-[#2D2D2E] transition-all text-[10px] uppercase tracking-widest group"
            >
              <Plus className="w-3 h-3 group-hover:rotate-90 transition-transform" /> New Agent
            </button>
          </div>
        </header>

        {/* Action Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
          <QuickCard emoji="🌞" title="Briefing" desc="Inbox + Slack status" />
          <QuickCard emoji="💬" title="Follow-up" desc="Lead management" />
          <QuickCard emoji="📋" title="Research" desc="LinkedIn + Web" />
          <QuickCard emoji="✉️" title="Triage" desc="Auto-draft email" />
        </div>

        {/* Input Zone - Centered Floating Feel */}
        <div className="flex-1 flex flex-col justify-center max-w-3xl mx-auto w-full">
          <div className="bg-white border border-gray-100 rounded-[2rem] p-8 md:p-12 text-center group hover:border-black transition-all duration-500 shadow-sm hover:shadow-2xl relative overflow-hidden">
            <div className="mx-auto w-12 h-12 bg-gray-50 rounded-xl flex items-center justify-center text-3xl mb-4 group-hover:rotate-6 transition-transform relative z-10">🐆</div>
            <h3 className="text-lg md:text-xl font-bold text-[#1C1C1E] mb-1 tracking-tight relative z-10 uppercase italic">Command Central</h3>
            <p className="text-[#6B7280] font-medium mb-8 text-[10px] uppercase tracking-[0.2em] relative z-10 opacity-50">Local Intelligence Active</p>
            
            <div className="flex flex-wrap gap-2 justify-center relative z-10">
              <ActionButton icon={<Zap className="w-3.5 h-3.5" />} label="Quick Start" />
              <ActionButton icon={<Target className="w-3.5 h-3.5" />} label="Advanced" />
              <ActionButton icon={<Plus className="w-3.5 h-3.5" />} label="Import Routine" />
            </div>
            
            <div className="absolute -right-16 -bottom-16 w-48 h-48 bg-gray-50 rounded-full blur-3xl opacity-40"></div>
          </div>
        </div>

        <footer className="mt-8 flex justify-center opacity-20">
          <div className="px-4 py-1.5 rounded-full bg-white border border-gray-100 text-[8px] font-bold uppercase tracking-[0.3em]">
            Hermes Local Execution • 0% Cloud
          </div>
        </footer>
      </main>
      
      {showConnect && <ConnectPanel onClose={() => setShowConnect(false)} />}
      {showBuilder && <AgentBuilder onClose={() => setShowBuilder(false)} />}
    </div>
  );
};

const NavItem: React.FC<{ icon: React.ReactNode, label: string, active?: boolean, onClick?: () => void }> = ({ icon, label, active, onClick }) => (
  <div 
    onClick={onClick}
    className={`flex items-center gap-3 px-4 py-2.5 rounded-xl font-bold cursor-pointer transition-all duration-300 ${
    active ? 'bg-black text-white shadow-md' : 'hover:bg-gray-50 text-[#6B7280] hover:text-black'
  }`}>
    {icon}
    <span className="hidden md:block text-[11px] uppercase tracking-wider">{label}</span>
  </div>
);

const AgentItem: React.FC<{ label: string, color: string, inactive?: boolean }> = ({ label, color, inactive }) => (
  <div className={`flex items-center gap-3 px-4 py-2 text-[10px] font-bold transition-all duration-300 cursor-pointer rounded-lg hover:bg-gray-50 uppercase tracking-widest ${
    inactive ? 'opacity-30' : 'text-[#374151]'
  }`}>
    <span className={`w-1.5 h-1.5 rounded-full ${color} ${!inactive && 'animate-pulse'}`}></span>
    {label}
  </div>
);

const QuickCard: React.FC<{ emoji: string, title: string, desc: string }> = ({ emoji, title, desc }) => (
  <div className="bg-white rounded-2xl p-6 hover:shadow-xl transition-all duration-500 cursor-pointer border border-gray-50 hover:border-black group relative overflow-hidden shadow-sm flex flex-col items-start min-h-[140px]">
    <div className="text-3xl mb-4 group-hover:scale-110 transition-transform">{emoji}</div>
    <div className="font-bold text-sm mb-1 text-[#1C1C1E] uppercase tracking-tight">{title}</div>
    <p className="text-[#6B7280] text-[9px] font-medium leading-normal uppercase tracking-wider opacity-60">{desc}</p>
    <div className="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity translate-x-2 group-hover:translate-x-0 duration-300 text-black">
      <ArrowRight className="w-3.5 h-3.5" />
    </div>
  </div>
);

const ActionButton: React.FC<{ icon: React.ReactNode, label: string }> = ({ icon, label }) => (
  <button className="px-5 py-2.5 bg-gray-50 hover:bg-black hover:text-white rounded-lg font-bold transition-all duration-300 border border-transparent text-[10px] uppercase tracking-widest flex items-center gap-2">
    {icon} {label}
  </button>
);

export default Dashboard;
