import { WorkflowDiagram } from './WorkflowDiagram';

export function HowItWorks() {
  return (
    <section className="py-24">
      <div className="container mx-auto px-6">
        <h2 className="text-3xl font-bold text-center text-text-primary mb-4">
          How It Works
        </h2>
        <p className="text-center text-text-secondary max-w-2xl mx-auto mb-12">
          jjj implements Popperian epistemology: knowledge grows through bold
          conjectures and rigorous criticism.
        </p>

        <WorkflowDiagram />

        <blockquote className="mt-12 max-w-2xl mx-auto border-l-4 border-accent pl-6 italic text-text-secondary">
          "The method of science is the method of bold conjectures and ingenious
          and severe attempts to refute them."
          <cite className="block mt-2 text-sm text-text-secondary/70 not-italic">
            — Karl Popper
          </cite>
        </blockquote>
      </div>
    </section>
  );
}
