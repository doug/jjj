import React from 'react';

/**
 * Scalable SVG tree diagram showing distributed collaboration:
 *   ROOT PROBLEM → [Solution A (Alice), Solution B (Bob)]
 *                → each gets a CRITIQUE
 *                → APPROVED ✓ and WITHDRAWN ✗
 *
 * Uses viewBox so it scales cleanly at any viewport width.
 * All fills use inline style (not presentation attributes) so CSS resets can't override them.
 */

// Node geometry helpers
const mid  = (x: number, w: number) => x + w / 2;
const bot  = (y: number, h: number) => y + h;

// Box dimensions & positions (top-left x/y)
const ROOT     = { x: 210, y: 20,  w: 180, h: 48 };
const SOL_A    = { x:  65, y: 140, w: 158, h: 44 };
const SOL_B    = { x: 377, y: 140, w: 158, h: 44 };
const CRIT_A   = { x:  74, y: 252, w: 140, h: 38 };
const CRIT_B   = { x: 386, y: 252, w: 140, h: 38 };
const ACCEPTED = { x:  72, y: 358, w: 148, h: 44 };
const REFUTED  = { x: 383, y: 358, w: 134, h: 44 };

// Centre-x of each node
const ROOT_CX     = mid(ROOT.x,     ROOT.w);     // 300
const SOL_A_CX    = mid(SOL_A.x,    SOL_A.w);    // 144
const SOL_B_CX    = mid(SOL_B.x,    SOL_B.w);    // 456
const CRIT_A_CX   = mid(CRIT_A.x,   CRIT_A.w);   // 144
const CRIT_B_CX   = mid(CRIT_B.x,   CRIT_B.w);   // 456
const ACCEPTED_CX = mid(ACCEPTED.x, ACCEPTED.w);  // 146
const REFUTED_CX  = mid(REFUTED.x,  REFUTED.w);   // 450

const WHITE       = { fill: 'white' } as const;
const WHITE_FAINT = { fill: 'rgba(255,255,255,0.72)' } as const;
const WHITE_MUTED = { fill: 'rgba(255,255,255,0.65)' } as const;

export function CollaborationDiagram() {
  return (
    <div className="w-full max-w-2xl mx-auto px-4">
      <svg
        viewBox="0 0 600 410"
        className="w-full h-auto"
        xmlns="http://www.w3.org/2000/svg"
        style={{ fontFamily: 'Inter, system-ui, sans-serif', color: 'var(--text-primary)' }}
        aria-label="Collaboration flow: a problem branches to two competing solutions; each is critiqued; one is approved and one is withdrawn"
      >
        <defs>
          {/* Single marker reused for all arrows */}
          <marker id="flow-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" style={{ fill: 'currentColor', opacity: 0.7 }} />
          </marker>
        </defs>

        {/* ── Connections ────────────────────────────────────────── */}

        {/* Root → Solutions (branching quadratic curves) */}
        <path
          d={`M ${ROOT_CX} ${bot(ROOT.y, ROOT.h)} Q ${(ROOT_CX + SOL_A_CX) / 2} ${bot(ROOT.y, ROOT.h) + 32} ${SOL_A_CX} ${SOL_A.y}`}
          style={{ fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55, strokeLinecap: 'round' }}
          markerEnd="url(#flow-arrow)"
        />
        <path
          d={`M ${ROOT_CX} ${bot(ROOT.y, ROOT.h)} Q ${(ROOT_CX + SOL_B_CX) / 2} ${bot(ROOT.y, ROOT.h) + 32} ${SOL_B_CX} ${SOL_B.y}`}
          style={{ fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55, strokeLinecap: 'round' }}
          markerEnd="url(#flow-arrow)"
        />

        {/* Solutions → Critiques */}
        <line
          x1={SOL_A_CX}  y1={bot(SOL_A.y,  SOL_A.h)}
          x2={CRIT_A_CX} y2={CRIT_A.y}
          style={{ stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55 }}
          markerEnd="url(#flow-arrow)"
        />
        <line
          x1={SOL_B_CX}  y1={bot(SOL_B.y,  SOL_B.h)}
          x2={CRIT_B_CX} y2={CRIT_B.y}
          style={{ stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55 }}
          markerEnd="url(#flow-arrow)"
        />

        {/* Critiques → Outcomes */}
        <line
          x1={CRIT_A_CX}   y1={bot(CRIT_A.y,   CRIT_A.h)}
          x2={ACCEPTED_CX}  y2={ACCEPTED.y}
          style={{ stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55 }}
          markerEnd="url(#flow-arrow)"
        />
        <line
          x1={CRIT_B_CX}  y1={bot(CRIT_B.y,  CRIT_B.h)}
          x2={REFUTED_CX} y2={REFUTED.y}
          style={{ stroke: 'currentColor', strokeWidth: 1.5, strokeOpacity: 0.55 }}
          markerEnd="url(#flow-arrow)"
        />

        {/* ── Nodes ─────────────────────────────────────────────── */}

        {/* ROOT PROBLEM */}
        <rect {...ROOT} rx="10" style={{ fill: '#ef4444' }} />
        <text x={ROOT_CX} y={ROOT.y + ROOT.h / 2} textAnchor="middle" dominantBaseline="middle"
          fontSize="13" fontWeight="800" style={WHITE}>
          PROBLEM
        </text>

        {/* SOLUTION A */}
        <rect {...SOL_A} rx="9" style={{ fill: '#3b82f6' }} />
        <text x={SOL_A_CX} y={SOL_A.y + SOL_A.h / 2 - 7} textAnchor="middle" dominantBaseline="middle"
          fontSize="11" fontWeight="800" style={WHITE}>
          SOLUTION A
        </text>
        <text x={SOL_A_CX} y={SOL_A.y + SOL_A.h / 2 + 8} textAnchor="middle" dominantBaseline="middle"
          fontSize="9" style={WHITE_FAINT}>
          (Alice)
        </text>

        {/* SOLUTION B */}
        <rect {...SOL_B} rx="9" style={{ fill: '#3b82f6' }} />
        <text x={SOL_B_CX} y={SOL_B.y + SOL_B.h / 2 - 7} textAnchor="middle" dominantBaseline="middle"
          fontSize="11" fontWeight="800" style={WHITE}>
          SOLUTION B
        </text>
        <text x={SOL_B_CX} y={SOL_B.y + SOL_B.h / 2 + 8} textAnchor="middle" dominantBaseline="middle"
          fontSize="9" style={WHITE_FAINT}>
          (Bob)
        </text>

        {/* CRITIQUE A */}
        <rect {...CRIT_A} rx="8" style={{ fill: '#a855f7' }} />
        <text x={CRIT_A_CX} y={CRIT_A.y + CRIT_A.h / 2} textAnchor="middle" dominantBaseline="middle"
          fontSize="11" fontWeight="800" style={WHITE}>
          CRITIQUE
        </text>

        {/* CRITIQUE B */}
        <rect {...CRIT_B} rx="8" style={{ fill: '#a855f7' }} />
        <text x={CRIT_B_CX} y={CRIT_B.y + CRIT_B.h / 2} textAnchor="middle" dominantBaseline="middle"
          fontSize="11" fontWeight="800" style={WHITE}>
          CRITIQUE
        </text>

        {/* APPROVED */}
        <rect {...ACCEPTED} rx="10" style={{ fill: '#22c55e' }} />
        <text x={ACCEPTED_CX} y={ACCEPTED.y + ACCEPTED.h / 2} textAnchor="middle" dominantBaseline="middle"
          fontSize="12" fontWeight="800" style={WHITE}>
          APPROVED ✓
        </text>

        {/* WITHDRAWN — muted to show it's a valid but terminal outcome */}
        <rect {...REFUTED} rx="10" style={{ fill: '#71717a' }} />
        <text x={REFUTED_CX} y={REFUTED.y + REFUTED.h / 2 - 7} textAnchor="middle" dominantBaseline="middle"
          fontSize="12" fontWeight="800" style={WHITE}>
          WITHDRAWN ✗
        </text>
        <text x={REFUTED_CX} y={REFUTED.y + REFUTED.h / 2 + 8} textAnchor="middle" dominantBaseline="middle"
          fontSize="8" style={WHITE_MUTED}>
          documented
        </text>
      </svg>

      {/* Info cards */}
      <div className="mt-8 grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="p-5 rounded-2xl border border-border/50 bg-surface/60">
          <h4 className="font-bold text-sm text-text-primary mb-2">Distributed Collaboration</h4>
          <p className="text-xs leading-relaxed text-text-secondary">
            Multiple people propose competing solutions for the same problem. Critiques happen asynchronously — only the most robust conjecture is approved.
          </p>
        </div>
        <div className="p-5 rounded-2xl border border-border/50 bg-surface/60">
          <h4 className="font-bold text-sm text-text-primary mb-2">Withdrawal is Progress</h4>
          <p className="text-xs leading-relaxed text-text-secondary">
            A withdrawn solution isn't wasted work — it's documented knowledge. The critique that caused it is preserved, preventing the same mistake twice.
          </p>
        </div>
      </div>
    </div>
  );
}
