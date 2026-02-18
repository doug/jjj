import { WorkflowDiagram } from './WorkflowDiagram';
import { CollaborationDiagram } from './CollaborationDiagram';

export function HowItWorks() {
  return (
    <section className="py-32 relative overflow-hidden">
      {/* Subtle Background Art */}
      <div className="absolute top-1/4 -right-24 w-96 h-96 bg-accent/5 blur-[120px] rounded-full pointer-events-none" />
      <div className="absolute bottom-1/4 -left-24 w-96 h-96 bg-purple-500/5 blur-[120px] rounded-full pointer-events-none" />

      <div className="container mx-auto px-6 relative">
        <div className="text-center mb-24">
          <h2 className="text-4xl md:text-5xl font-black text-text-primary mb-6 tracking-tight">
            How It Works
          </h2>
          <p className="text-lg text-text-secondary max-w-2xl mx-auto leading-relaxed">
            jjj implements <span className="text-accent font-semibold">Popperian epistemology</span>: 
            knowledge grows through bold conjectures and rigorous criticism.
          </p>
        </div>

        <div className="space-y-32">
          <div className="relative">
            <div className="absolute inset-x-0 -top-8 flex justify-center">
              <span className="px-4 py-1 bg-background border border-border rounded-full text-[10px] font-black uppercase tracking-[0.3em] text-text-secondary/60">
                Micro Flow
              </span>
            </div>
            <h3 className="text-2xl font-bold text-center text-text-primary mb-12">
              The Evolution of an Idea
            </h3>
            <WorkflowDiagram />
          </div>

          <div className="relative pt-24 border-t border-border/40">
            <div className="absolute inset-x-0 -top-4 flex justify-center">
              <span className="px-4 py-1 bg-background border border-border rounded-full text-[10px] font-black uppercase tracking-[0.3em] text-text-secondary/60">
                Macro Flow
              </span>
            </div>
            <h3 className="text-2xl font-bold text-center text-text-primary mb-12">
              Distributed Collaboration & Nested Problems
            </h3>
            <CollaborationDiagram />
          </div>
        </div>

        <div className="mt-32 max-w-3xl mx-auto">
          <blockquote className="relative p-12 bg-white/5 backdrop-blur-sm rounded-[2rem] border border-white/10 italic text-text-secondary text-lg leading-relaxed shadow-2xl">
            <span className="absolute top-4 left-6 text-7xl text-accent/20 font-serif leading-none">“</span>
            "The method of science is the method of bold conjectures and ingenious
            and severe attempts to refute them."
            <cite className="block mt-6 text-base text-text-primary/70 not-italic font-bold">
              — Karl Popper, <span className="font-normal opacity-60 italic">Objective Knowledge</span>
            </cite>
          </blockquote>
        </div>
      </div>
    </section>
  );
}
