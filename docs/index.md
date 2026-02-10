# jjj

<div class="hero">
  <h1>Distributed Project Management</h1>
  <p class="subtitle">Problems, solutions, and critiques — all in your repo</p>
  <div class="hero-buttons">
    <a href="getting-started/installation.html" class="btn btn-primary">Get Started</a>
    <a href="https://github.com/doug/jjj" class="btn btn-secondary">View on GitHub</a>
  </div>
</div>

<div class="diagrams">
  <div class="diagram">
    <h3>The Refinement Cycle</h3>
    <svg viewBox="0 0 400 200" xmlns="http://www.w3.org/2000/svg">
      <!-- Problem node -->
      <rect x="20" y="70" width="100" height="60" rx="8" fill="#e74c3c" opacity="0.9"/>
      <text x="70" y="105" text-anchor="middle" fill="white" font-weight="bold" font-size="14">PROBLEM</text>

      <!-- Solution node -->
      <rect x="150" y="70" width="100" height="60" rx="8" fill="#3498db" opacity="0.9"/>
      <text x="200" y="105" text-anchor="middle" fill="white" font-weight="bold" font-size="14">SOLUTION</text>

      <!-- Critique node -->
      <rect x="280" y="70" width="100" height="60" rx="8" fill="#9b59b6" opacity="0.9"/>
      <text x="330" y="105" text-anchor="middle" fill="white" font-weight="bold" font-size="14">CRITIQUE</text>

      <!-- Arrows -->
      <defs>
        <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
          <polygon points="0 0, 10 3.5, 0 7" fill="#666"/>
        </marker>
      </defs>

      <!-- Problem to Solution -->
      <line x1="120" y1="100" x2="145" y2="100" stroke="#666" stroke-width="2" marker-end="url(#arrowhead)"/>

      <!-- Solution to Critique -->
      <line x1="250" y1="100" x2="275" y2="100" stroke="#666" stroke-width="2" marker-end="url(#arrowhead)"/>

      <!-- Refine loop back -->
      <path d="M 330 130 Q 330 170 200 170 Q 70 170 70 135" stroke="#27ae60" stroke-width="2" fill="none" marker-end="url(#arrowhead)"/>
      <text x="200" y="185" text-anchor="middle" fill="#27ae60" font-size="12" font-style="italic">refine</text>
    </svg>
  </div>

  <div class="diagram">
    <h3>Roadmap &amp; Structure</h3>
    <svg viewBox="0 0 400 200" xmlns="http://www.w3.org/2000/svg">
      <!-- Timeline -->
      <line x1="30" y1="50" x2="370" y2="50" stroke="#666" stroke-width="3"/>

      <!-- Milestone markers -->
      <circle cx="80" cy="50" r="12" fill="#f39c12"/>
      <text x="80" y="55" text-anchor="middle" fill="white" font-weight="bold" font-size="10">M1</text>
      <text x="80" y="30" text-anchor="middle" fill="#666" font-size="10">v1.0</text>

      <circle cx="200" cy="50" r="12" fill="#f39c12"/>
      <text x="200" y="55" text-anchor="middle" fill="white" font-weight="bold" font-size="10">M2</text>
      <text x="200" y="30" text-anchor="middle" fill="#666" font-size="10">v2.0</text>

      <circle cx="320" cy="50" r="12" fill="#f39c12"/>
      <text x="320" y="55" text-anchor="middle" fill="white" font-weight="bold" font-size="10">M3</text>
      <text x="320" y="30" text-anchor="middle" fill="#666" font-size="10">v3.0</text>

      <!-- Problems hanging from milestones -->
      <line x1="60" y1="62" x2="60" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="40" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="60" y="107" text-anchor="middle" fill="white" font-size="9">P1</text>

      <line x1="100" y1="62" x2="100" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="80" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="100" y="107" text-anchor="middle" fill="white" font-size="9">P2</text>

      <line x1="180" y1="62" x2="180" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="160" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="180" y="107" text-anchor="middle" fill="white" font-size="9">P3</text>

      <line x1="220" y1="62" x2="220" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="200" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="220" y="107" text-anchor="middle" fill="white" font-size="9">P4</text>

      <line x1="300" y1="62" x2="300" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="280" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="300" y="107" text-anchor="middle" fill="white" font-size="9">P5</text>

      <line x1="340" y1="62" x2="340" y2="90" stroke="#ccc" stroke-width="1"/>
      <rect x="320" y="90" width="40" height="25" rx="4" fill="#e74c3c" opacity="0.8"/>
      <text x="340" y="107" text-anchor="middle" fill="white" font-size="9">P6</text>

      <!-- Solutions attached to problems -->
      <line x1="60" y1="115" x2="60" y2="140" stroke="#ccc" stroke-width="1"/>
      <rect x="45" y="140" width="30" height="20" rx="3" fill="#3498db" opacity="0.8"/>
      <text x="60" y="154" text-anchor="middle" fill="white" font-size="8">S1</text>

      <line x1="180" y1="115" x2="180" y2="140" stroke="#ccc" stroke-width="1"/>
      <rect x="165" y="140" width="30" height="20" rx="3" fill="#3498db" opacity="0.8"/>
      <text x="180" y="154" text-anchor="middle" fill="white" font-size="8">S2</text>
    </svg>
  </div>
</div>

<div class="terminal">
  <div class="terminal-header">
    <span class="terminal-dot red"></span>
    <span class="terminal-dot yellow"></span>
    <span class="terminal-dot green"></span>
    <span class="terminal-title">Collaboration in Action</span>
  </div>
  <div class="terminal-body">
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj init</span>
    </div>
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj problem new "Search is slow" --priority P1</span>
    </div>
    <div class="terminal-output">Created problem 01957d: Search is slow</div>
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj push</span>
    </div>
    <div class="terminal-divider"></div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj fetch</span>
    </div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj solution new "Add search index" --problem "Search is slow"</span>
    </div>
    <div class="terminal-output">Created solution 01958a: Add search index</div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj push</span>
    </div>
    <div class="terminal-divider"></div>
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj fetch</span>
    </div>
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj critique new "search index" "Missing error handling"</span>
    </div>
    <div class="terminal-output">Created critique 01958b: Missing error handling</div>
    <div class="terminal-section">
      <span class="terminal-user">alice</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj push</span>
    </div>
    <div class="terminal-divider"></div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj fetch</span>
    </div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj critique address "error handling"</span>
    </div>
    <div class="terminal-output">Addressed critique 01958b</div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj solution accept "search index"</span>
    </div>
    <div class="terminal-output">Accepted solution 01958a</div>
    <div class="terminal-section">
      <span class="terminal-user">bob</span>
      <span class="terminal-prompt">$</span>
      <span class="terminal-cmd">jjj push</span>
    </div>
  </div>
</div>

<div class="features">
  <div class="feature-card">
    <div class="feature-icon">📡</div>
    <h3>Offline First</h3>
    <p>All metadata lives in your repo. Works on a plane.</p>
  </div>

  <div class="feature-card">
    <div class="feature-icon">🔀</div>
    <h3>Survives Rebases</h3>
    <p>Change IDs persist across history rewrites.</p>
  </div>

  <div class="feature-card">
    <div class="feature-icon">💬</div>
    <h3>Critique Driven</h3>
    <p>Solutions must survive criticism before acceptance.</p>
  </div>

  <div class="feature-card">
    <div class="feature-icon">📦</div>
    <h3>No Server Required</h3>
    <p>Sync via standard git push/pull.</p>
  </div>

  <div class="feature-card">
    <div class="feature-icon">🤖</div>
    <h3>AI Agent Native</h3>
    <p>CLI and text files work seamlessly with AI assistants.</p>
  </div>
</div>

<div class="comparison">
  <h2>How jjj Compares</h2>
  <p class="subtitle">jjj vs hosted project management tools</p>

  <table>
    <thead>
      <tr>
        <th>Feature</th>
        <th>jjj</th>
        <th>GitHub Issues</th>
        <th>Linear</th>
        <th>Jira</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Works offline</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>No server required</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Survives rebases</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Data in your repo</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Structured critiques</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>AI agent native</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Built for Jujutsu</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Team collaboration</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td>VS Code extension</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td>Terminal TUI</td>
        <td class="check">✓</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
        <td class="cross">✗</td>
      </tr>
      <tr>
        <td>Web UI</td>
        <td class="cross">✗</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
      <tr>
        <td>Mobile app</td>
        <td class="cross">✗</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
        <td class="check">✓</td>
      </tr>
    </tbody>
  </table>
</div>

<div class="philosophy">
  <h2>Why Popperian?</h2>
  <p class="intro">jjj is built on Karl Popper's theory of knowledge growth: we advance by proposing bold conjectures and subjecting them to rigorous criticism.</p>

  <blockquote>
    "The method of science is the method of bold conjectures and ingenious and severe attempts to refute them."
    <cite>— Karl Popper</cite>
  </blockquote>

  <ul>
    <li><strong>Problems</strong> are explicit. You can't solve what you haven't articulated.</li>
    <li><strong>Solutions</strong> are conjectures. They're tentative, never final, always open to criticism.</li>
    <li><strong>Critiques</strong> are how we grow. Error elimination drives progress, not authority.</li>
  </ul>

  <p class="closing">This isn't bureaucracy. It's intellectual honesty encoded in your workflow.</p>

  <p class="book-link">Learn more: <a href="https://www.amazon.com/Conjectures-Refutations-Scientific-Knowledge-Routledge/dp/0415285941">Conjectures and Refutations</a> by Karl Popper</p>
</div>

## Getting Started

Ready to try jjj? Start with the [Installation](getting-started/installation.md) guide, then follow the [Quick Start](getting-started/quick-start.md) tutorial to create your first project.

## Learn More

- [Design Philosophy](architecture/design-philosophy.md) — the ideas behind jjj
- [CLI Reference](reference/cli-workflow.md) — complete command documentation
- [VS Code Extension](reference/vscode-extension.md) — graphical interface for jjj
