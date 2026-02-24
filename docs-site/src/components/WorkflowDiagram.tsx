import React from 'react';
import { Target, Zap, ShieldAlert } from 'lucide-react';

function ArrowRight() {
  return (
    <svg viewBox="0 0 48 20" width="48" height="20" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <line x1="2" y1="10" x2="35" y2="10"
        stroke="var(--accent)" strokeWidth="2" strokeDasharray="4 3" opacity="0.5" />
      <polygon points="32,5 46,10 32,15" fill="var(--accent)" opacity="0.5" />
    </svg>
  );
}

function ArrowDown() {
  return (
    <svg viewBox="0 0 20 44" width="20" height="44" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <line x1="10" y1="2" x2="10" y2="33"
        stroke="var(--accent)" strokeWidth="2" strokeDasharray="4 3" opacity="0.5" />
      <polygon points="5,29 10,43 15,29" fill="var(--accent)" opacity="0.5" />
    </svg>
  );
}

export function WorkflowDiagram() {
  return (
    <div className="max-w-3xl mx-auto py-8 px-4">
      {/* Three-phase flow */}
      <div className="flex flex-col md:flex-row items-center justify-center">

        {/* PROBLEM */}
        <div className="group relative flex flex-col items-center">
          <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-red-500 to-red-700 dark:from-red-600 dark:to-red-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105">
            <Target className="w-8 h-8 mb-2 opacity-90" />
            <span className="font-black text-lg tracking-tighter uppercase">PROBLEM</span>
          </div>
          <div className="mt-3 text-center">
            <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-70">Phase 1</p>
            <p className="text-sm text-text-secondary">Identify &amp; articulate</p>
          </div>
          <div className="absolute inset-0 bg-red-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity -z-10" />
        </div>

        {/* Connector */}
        <div className="my-4 md:my-0 md:mx-3 self-center flex items-center justify-center">
          <span className="hidden md:block"><ArrowRight /></span>
          <span className="md:hidden block"><ArrowDown /></span>
        </div>

        {/* SOLUTION */}
        <div className="group relative flex flex-col items-center">
          <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-blue-500 to-blue-700 dark:from-blue-600 dark:to-blue-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105">
            <Zap className="w-8 h-8 mb-2 opacity-90" />
            <span className="font-black text-lg tracking-tighter uppercase">SOLUTION</span>
          </div>
          <div className="mt-3 text-center">
            <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-70">Phase 2</p>
            <p className="text-sm text-text-secondary">Propose conjecture</p>
          </div>
          <div className="absolute inset-0 bg-blue-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity -z-10" />
        </div>

        {/* Connector */}
        <div className="my-4 md:my-0 md:mx-3 self-center flex items-center justify-center">
          <span className="hidden md:block"><ArrowRight /></span>
          <span className="md:hidden block"><ArrowDown /></span>
        </div>

        {/* CRITIQUE */}
        <div className="group relative flex flex-col items-center">
          <div className="w-36 h-28 rounded-2xl bg-linear-to-br from-purple-500 to-purple-700 dark:from-purple-600 dark:to-purple-900 border border-white/20 shadow-xl flex flex-col items-center justify-center text-white z-10 transition-all duration-300 group-hover:scale-105">
            <ShieldAlert className="w-8 h-8 mb-2 opacity-90" />
            <span className="font-black text-lg tracking-tighter uppercase">CRITIQUE</span>
          </div>
          <div className="mt-3 text-center">
            <p className="text-xs font-bold text-text-primary uppercase tracking-widest opacity-70">Phase 3</p>
            <p className="text-sm text-text-secondary">Eliminate errors</p>
          </div>
          <div className="absolute inset-0 bg-purple-500/20 blur-xl rounded-full opacity-0 group-hover:opacity-100 transition-opacity -z-10" />
        </div>
      </div>

      {/* Refine feedback loop — desktop arc, mobile label */}
      <div className="mt-2">
        {/*
          Arc spans from x=72 (center of leftmost card) to x=508 (center of rightmost card)
          within a 580-wide viewBox, matching the ~576px max-width of the flex row.
        */}
        <svg
          viewBox="0 0 580 56"
          className="hidden md:block w-full max-w-xl mx-auto h-auto"
          xmlns="http://www.w3.org/2000/svg"
          aria-label="Refine or refute loop"
        >
          <circle cx="72"  cy="8" r="3" fill="var(--accent)" opacity="0.4" />
          <path
            d="M 72 8 Q 290 52 508 8"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.5"
            strokeDasharray="5 4"
            opacity="0.35"
            strokeLinecap="round"
          />
          <circle cx="508" cy="8" r="3" fill="var(--accent)" opacity="0.4" />
          <text
            x="290" y="49"
            textAnchor="middle"
            fontSize="10"
            fill="var(--accent)"
            opacity="0.45"
            fontWeight="700"
            fontFamily="Inter, system-ui, sans-serif"
          >
            REFINE OR REFUTE
          </text>
        </svg>
        <p
          className="md:hidden text-center text-xs font-bold uppercase tracking-widest mt-3"
          style={{ color: 'var(--accent)', opacity: 0.45 }}
        >
          ↑ Refine or refute ↑
        </p>
      </div>
    </div>
  );
}
