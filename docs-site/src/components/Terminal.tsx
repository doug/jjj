interface TerminalLine {
  type: 'command' | 'output';
  user?: string;
  content: string;
}

interface TerminalProps {
  title?: string;
  lines: TerminalLine[];
}

export function Terminal({ title = 'Terminal', lines }: TerminalProps) {
  return (
    <div className="rounded-xl overflow-hidden border border-border shadow-lg">
      {/* Title bar */}
      <div className="bg-[#2D2A26] px-4 py-3 flex items-center gap-2">
        <div className="flex gap-2">
          <div className="w-3 h-3 rounded-full bg-[#ff5f56]" />
          <div className="w-3 h-3 rounded-full bg-[#ffbd2e]" />
          <div className="w-3 h-3 rounded-full bg-[#27c93f]" />
        </div>
        <span className="ml-3 text-sm text-gray-400 font-mono">{title}</span>
      </div>

      {/* Terminal content */}
      <div className="bg-[#1E1E1E] p-6 font-mono text-sm leading-relaxed">
        {lines.map((line, i) => (
          <div key={i} className="mb-1">
            {line.type === 'command' ? (
              <div>
                <span className="text-accent">{line.user || 'user'}</span>
                <span className="text-gray-500"> $ </span>
                <span className="text-gray-100">{line.content}</span>
              </div>
            ) : (
              <div className="text-gray-400 pl-0">{line.content}</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
