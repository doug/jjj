import React from 'react';
import { Share2, GitBranch, Target, AlertTriangle, CheckCircle2, RefreshCw } from 'lucide-react';

export function CollaborationDiagram() {
  return (
    <div className="w-full max-w-4xl mx-auto p-8 bg-background-secondary/20 backdrop-blur-xl rounded-[2.5rem] border border-border/50 overflow-hidden relative group">
      {/* Background Decorative Elements */}
      <div className="absolute top-0 right-0 w-64 h-64 bg-accent/5 blur-[100px] -z-10 rounded-full animate-pulse" />
      <div className="absolute bottom-0 left-0 w-64 h-64 bg-purple-500/5 blur-[100px] -z-10 rounded-full animate-pulse" style={{ animationDelay: '2s' }} />

      <div className="relative h-[530px] w-full">
        {/* fluid SVG Connections */}
        <svg className="absolute inset-0 w-full h-full pointer-events-none" xmlns="http://www.w3.org/2000/svg">
          <defs>
            <linearGradient id="collab-grad" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.4" />
              <stop offset="100%" stopColor="var(--accent)" stopOpacity="0" />
            </linearGradient>
            <filter id="glow">
              <feGaussianBlur stdDeviation="2" result="coloredBlur"/>
              <feMerge>
                <feMergeNode in="coloredBlur"/>
                <feMergeNode in="SourceGraphic"/>
              </feMerge>
            </filter>
            <marker id="collab-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
              <polygon points="0 0, 8 3, 0 6" fill="var(--accent)" opacity="0.6" />
            </marker>
            <marker id="red-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
              <polygon points="0 0, 8 3, 0 6" fill="#ef4444" opacity="0.6" />
            </marker>
          </defs>
          
          {/* Main Problem to Solutions - Curved */}
          <path d="M 50% 60 Q 40% 100 30% 140" fill="none" stroke="url(#collab-grad)" strokeWidth="2" strokeDasharray="6 4" marker-end="url(#collab-arrow)" className="animate-dash" strokeLinecap="round" />
          <path d="M 50% 60 Q 60% 100 70% 140" fill="none" stroke="url(#collab-grad)" strokeWidth="2" strokeDasharray="6 4" marker-end="url(#collab-arrow)" className="animate-dash" strokeLinecap="round" />
          
          {/* Branching Solutions to Critiques */}
          <path d="M 28% 180 Q 23% 210 19% 240" fill="none" stroke="#a855f7" strokeWidth="1.5" className="opacity-30" strokeLinecap="round" />
          <path d="M 72% 180 Q 77% 210 81% 240" fill="none" stroke="#a855f7" strokeWidth="1.5" className="opacity-30" strokeLinecap="round" />
          
          {/* Nested Problem Connection */}
          <path d="M 30% 180 Q 35% 230 40% 270" fill="none" stroke="#ef4444" strokeWidth="1.5" strokeDasharray="4 2" marker-end="url(#red-arrow)" className="opacity-40" />
          <circle cx="40%" cy="280" r="3" fill="#ef4444" filter="url(#glow)" />
          
          {/* Convergence to Emergent Knowledge */}
          <path d="M 22% 300 C 22% 400 50% 400 50% 430" fill="none" stroke="#10b981" strokeWidth="2" className="opacity-20" />
          <path d="M 78% 300 C 78% 400 50% 400 50% 430" fill="none" stroke="#10b981" strokeWidth="2" className="opacity-20" />
        </svg>

        {/* Root Problem */}
        <div className="absolute top-0 left-1/2 -translate-x-1/2 flex flex-col items-center z-20">
          <div className="px-6 py-4 bg-linear-to-br from-red-500 to-red-700 rounded-2xl shadow-[0_0_30px_rgba(239,68,68,0.3)] border border-white/20 transition-transform hover:scale-105">
            <Target className="w-8 h-8 text-white mb-1 mx-auto" filter="drop-shadow(0 2px 4px rgba(0,0,0,0.2))" />
            <span className="text-white font-black block text-center text-sm tracking-widest leading-none">ROOT PROBLEM</span>
          </div>
          <div className="mt-3 px-3 py-1 bg-black/40 backdrop-blur-md rounded-full border border-white/10 text-[9px] font-mono text-red-200">ID: alpha-902</div>
        </div>

        {/* User Alice Solution */}
        <div className="absolute top-[140px] left-[18%] flex flex-col items-center">
          <div className="px-5 py-4 bg-linear-to-br from-blue-500 to-blue-700 rounded-2xl shadow-xl border border-white/20 transition-all hover:-translate-y-1 hover:shadow-blue-500/30">
            <Share2 className="w-6 h-6 text-white mb-1 mx-auto" />
            <span className="text-white font-bold block text-[10px] tracking-wide text-center leading-none">ALICE'S PROPOSAL</span>
          </div>
          <div className="mt-2 text-[9px] font-mono text-blue-400 bg-blue-500/10 px-2 py-0.5 rounded border border-blue-500/20">conjecture-1</div>
        </div>

        {/* User Bob Solution */}
        <div className="absolute top-[140px] left-[62%] flex flex-col items-center">
          <div className="px-5 py-4 bg-linear-to-br from-blue-500 to-blue-700 rounded-2xl shadow-xl border border-white/20 transition-all hover:-translate-y-1 hover:shadow-blue-500/30">
            <Share2 className="w-6 h-6 text-white mb-1 mx-auto" />
            <span className="text-white font-bold block text-[10px] tracking-wide text-center leading-none">BOB'S PROPOSAL</span>
          </div>
          <div className="mt-2 text-[9px] font-mono text-blue-400 bg-blue-500/10 px-2 py-0.5 rounded border border-blue-500/20">conjecture-2</div>
        </div>

        {/* Critique Labels - Floating Glass */}
        <div className="absolute top-[260px] left-[8%] group/item">
          <div className="p-3 bg-purple-600/20 backdrop-blur-md border border-purple-500/40 rounded-xl shadow-lg transition-all group-hover/item:bg-purple-600/40 group-hover/item:scale-110">
            <AlertTriangle className="w-5 h-5 text-purple-400" />
          </div>
          <div className="absolute -bottom-6 left-1/2 -translate-x-1/2 whitespace-nowrap text-[8px] font-black text-purple-400 uppercase tracking-tighter">Hard Refutation</div>
        </div>

        <div className="absolute top-[260px] left-[80%] group/item">
          <div className="p-3 bg-purple-600/20 backdrop-blur-md border border-purple-500/40 rounded-xl shadow-lg transition-all group-hover/item:bg-purple-600/40 group-hover/item:scale-110">
            <AlertTriangle className="w-5 h-5 text-purple-400" />
          </div>
          <div className="absolute -bottom-6 left-1/2 -translate-x-1/2 whitespace-nowrap text-[8px] font-black text-purple-400 uppercase tracking-tighter">Structural Flaw</div>
        </div>

        {/* Nested Problem - Glitchy/Pulse Effect */}
        <div className="absolute top-[300px] left-[34%] flex flex-col items-center">
          <div className="px-4 py-2 bg-red-500/10 backdrop-blur-md border border-red-500/30 rounded-xl flex items-center gap-2 animate-pulse shadow-[0_0_15px_rgba(239,68,68,0.1)]">
            <div className="w-2 h-2 bg-red-500 rounded-full animate-ping" />
            <GitBranch className="w-4 h-4 text-red-500" />
            <span className="text-[10px] text-red-500 font-black uppercase tracking-widest">Nested Problem</span>
          </div>
          <span className="mt-2 text-[8px] text-text-secondary font-medium tracking-wide">Recursive Discovery</span>
        </div>

        {/* Emergent Knowledge - Final Boss */}
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex flex-col items-center z-20 w-full px-4">
          <div className="p-6 bg-linear-to-br from-green-500 to-emerald-700 rounded-[2rem] shadow-[0_20px_50px_rgba(16,185,129,0.3)] border border-white/30 transition-all duration-500 hover:scale-105 hover:shadow-green-500/50 cursor-pointer max-w-sm mx-auto w-full">
            <CheckCircle2 className="w-12 h-12 text-white mx-auto drop-shadow-lg" />
            <div className="mt-2 text-white text-center">
              <span className="block text-xs font-bold opacity-80 uppercase tracking-[0.2em] mb-1 leading-none">State: Resolved</span>
              <span className="font-black text-lg tracking-tight uppercase leading-tight mt-1 inline-block">EMERGENT KNOWLEDGE</span>
            </div>
          </div>
          <div className="mt-4 flex items-center gap-3 text-green-500 font-bold text-xs uppercase tracking-[0.3em] overflow-hidden">
            <RefreshCw className="w-4 h-4 animate-spin-slow" />
            <span>Successive Refinement</span>
          </div>
        </div>
      </div>
      
      {/* Information Cards */}
      <div className="mt-16 grid grid-cols-1 md:grid-cols-2 gap-8 px-6 pb-4">
        <div className="relative p-6 bg-white/5 backdrop-blur-sm rounded-3xl border border-white/10 group/card transition-colors hover:bg-white/10">
          <div className="absolute -top-4 -left-4 w-12 h-12 bg-accent/20 rounded-2xl flex items-center justify-center border border-accent/30 group-hover/card:scale-110 transition-transform">
            <Share2 className="w-6 h-6 text-accent" />
          </div>
          <h4 className="font-black text-sm text-text-primary uppercase tracking-widest mb-3 ml-8">
            Distributed Collaboration
          </h4>
          <p className="text-xs leading-relaxed text-text-secondary">
            Alice, Bob, and others propose competing conjectures. Error elimination via critique happens asynchronously and locally, ensuring only the most robust ideas survive.
          </p>
        </div>
        
        <div className="relative p-6 bg-white/5 backdrop-blur-sm rounded-3xl border border-white/10 group/card transition-colors hover:bg-white/10">
          <div className="absolute -top-4 -left-4 w-12 h-12 bg-red-500/20 rounded-2xl flex items-center justify-center border border-red-500/30 group-hover/card:scale-110 transition-transform">
            <GitBranch className="w-6 h-6 text-red-500" />
          </div>
          <h4 className="font-black text-sm text-text-primary uppercase tracking-widest mb-3 ml-8">
            Recursive Problems
          </h4>
          <p className="text-xs leading-relaxed text-text-secondary">
            Solving one problem often reveals deeper questions. jjj tracks these sub-problems as nested entities, allowing for infinite depth in project exploration.
          </p>
        </div>
      </div>
    </div>
  );
}
