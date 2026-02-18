import React from 'react';
import { Target, Zap, ShieldAlert, ChevronRight } from 'lucide-react';

export function WorkflowDiagram() {
  return (
    <div className="relative flex flex-col md:flex-row items-center justify-center gap-12 md:gap-4 py-12 max-w-4xl mx-auto">
      {/* Background Glow */}
      <div className="absolute inset-0 bg-accent/5 blur-3xl rounded-full -z-10" />

      {/* SVG Connections (Desktop) */}
      <svg className="absolute inset-0 w-full h-full hidden md:block pointer-events-none" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="line-grad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.2" />
            <stop offset="50%" stopColor="var(--accent)" stopOpacity="1" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.2" />
          </linearGradient>
          <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
            <polygon points="0 0, 10 3.5, 0 7" fill="var(--accent)" />
          </marker>
        </defs>
        <path d="M 160 80 L 250 80" stroke="url(#line-grad)" strokeWidth="2" strokeDasharray="4 4" marker-end="url(#arrowhead)" className="animate-dash" />
        <path d="M 420 80 L 510 80" stroke="url(#line-grad)" strokeWidth="2" strokeDasharray="4 4" marker-end="url(#arrowhead)" className="animate-dash" />
      </svg>

      {/* Problem */}
      <div className="group relative flex flex-col items-center">
        <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-red-500 to-red-700 dark:from-red-600 dark:to-red-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105 group-hover:shadow-red-500/20">
          <Target className="w-8 h-8 mb-2 opacity-90" />
          <span className="font-black text-lg tracking-tighter uppercase">PROBLEM</span>
        </div>
        <div className="mt-4 text-center">
          <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-80">Phase 1</p>
          <p className="text-sm text-text-secondary">Identify & articulate</p>
        </div>
        {/* Glow behind */}
        <div className="absolute inset-0 bg-red-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>

      {/* Mobile Arrow */}
      <ChevronRight className="md:hidden w-8 h-8 text-accent animate-bounce rotate-90" />

      {/* Solution */}
      <div className="group relative flex flex-col items-center">
        <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-blue-500 to-blue-700 dark:from-blue-600 dark:to-blue-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105 group-hover:shadow-blue-500/20">
          <Zap className="w-8 h-8 mb-2 opacity-90" />
          <span className="font-black text-lg tracking-tighter uppercase">SOLUTION</span>
        </div>
        <div className="mt-4 text-center">
          <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-80">Phase 2</p>
          <p className="text-sm text-text-secondary">Propose conjecture</p>
        </div>
        <div className="absolute inset-0 bg-blue-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>

      {/* Mobile Arrow */}
      <ChevronRight className="md:hidden w-8 h-8 text-accent animate-bounce rotate-90" />

      {/* Critique */}
      <div className="group relative flex flex-col items-center">
        <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-purple-500 to-purple-700 dark:from-purple-600 dark:to-purple-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105 group-hover:shadow-purple-500/20">
          <ShieldAlert className="w-8 h-8 mb-2 opacity-90" />
          <span className="font-black text-lg tracking-tighter uppercase">CRITIQUE</span>
        </div>
        <div className="mt-4 text-center">
          <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-80">Phase 3</p>
          <p className="text-sm text-text-secondary">Eliminate errors</p>
        </div>
        <div className="absolute inset-0 bg-purple-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>

      {/* Refine loop (Desktop) */}
      <div className="hidden md:flex flex-col items-center ml-8 group">
        <div className="w-16 h-16 rounded-full border-2 border-dashed border-success/40 flex items-center justify-center transition-all group-hover:rotate-180 group-hover:border-success">
          <span className="text-success text-3xl">↻</span>
        </div>
        <p className="mt-2 text-xs font-bold text-success uppercase tracking-widest">Refine</p>
      </div>
    </div>
  );
}
