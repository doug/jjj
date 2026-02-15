export function WorkflowDiagram() {
  return (
    <div className="flex flex-col md:flex-row items-center justify-center gap-4 md:gap-8 py-8">
      {/* Problem */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-red-600 dark:bg-red-900 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          PROBLEM
        </div>
        <p className="mt-2 text-sm text-text-secondary">Identify & articulate</p>
      </div>

      {/* Arrow */}
      <div className="text-2xl text-accent hidden md:block">→</div>
      <div className="text-2xl text-accent md:hidden">↓</div>

      {/* Solution */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-blue-600 dark:bg-blue-900 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          SOLUTION
        </div>
        <p className="mt-2 text-sm text-text-secondary">Propose conjecture</p>
      </div>

      {/* Arrow */}
      <div className="text-2xl text-accent hidden md:block">→</div>
      <div className="text-2xl text-accent md:hidden">↓</div>

      {/* Critique */}
      <div className="flex flex-col items-center">
        <div className="w-32 h-24 rounded-xl bg-purple-600 dark:bg-purple-900 flex items-center justify-center text-white font-bold text-lg shadow-lg">
          CRITIQUE
        </div>
        <p className="mt-2 text-sm text-text-secondary">Eliminate errors</p>
      </div>

      {/* Refine loop */}
      <div className="hidden md:flex flex-col items-center ml-4">
        <div className="text-success text-xl">↻</div>
        <p className="text-sm text-success italic">refine</p>
      </div>
    </div>
  );
}
