<!--
The front page of the documentation site (decision AI2). Its wording comes
from docs/design/08-positioning.md (sections 2 and 6) and from the top of
README.md; tools/site.py gives it the layout of `main.home`.
-->

<section class="hero">
<h1>The scripting language of AI&nbsp;agents.</h1>
<p class="tagline">The agent writes it, you review it at a glance, and the program can only do what it declares.</p>
<p class="lead">A program declares what it may do, down to the path, the host and the budget, and the runtime admits only that, dependencies included. Every run can be recorded, replayed and narrated, and the syntax is plain English so that a person reviews a program in one pass.</p>
<p class="actions"><a class="button primary" href="#install">Install</a> <a class="button" href="cheatsheet.md">The cheat sheet</a> <a class="button" href="https://github.com/renyi-lang/renyi">GitHub</a></p>
</section>

<section class="sample">
<div>
<h2>A signature</h2>
<p class="statement">What a program may do is in its signature.</p>
<p>A reviewer who has never seen Renyi reads what a program touches from its signature. The compiler refuses what is not declared; the runtime admits only that.</p>
</div>
<pre><code><span class="k">public function</span> active_adult_emails(path: Path)
  <span class="k">returns</span> List <span class="k">of</span> Email
  <span class="k">or fails with</span> FileError <span class="k">or</span> JsonError
  <span class="k">needs</span> filesystem.read
  <span class="k">purpose:</span><span class="c"> Read users from a JSON file and return the e-mails of active users who are at least 18.</span>

  <span class="k">let</span> text <span class="k">be</span> filesystem.read_text(path) <span class="k">otherwise fail</span>
  <span class="k">let</span> users: List <span class="k">of</span> User <span class="k">be</span> json.parse(text) <span class="k">otherwise fail</span>
  <span class="k">let</span> emails <span class="k">be</span>
    <span class="k">for each</span> user <span class="k">in</span> users
    <span class="k">where</span> user.is_active <span class="k">and</span> user.age <span class="k">is at least</span> 18
    <span class="k">sorted by</span> user.name
    <span class="k">collect</span> user.email
  <span class="k">return</span> emails
<span class="k">end</span></code></pre>
</section>

<section>
<h2>What Renyi leads with</h2>
<div class="points">
<div><span class="n">01</span><h3>Effects are capabilities</h3><p>Every function declares its effects, scoped to a path or a host, budgeted (<code>at most 60 per minute</code>) and guarded (<code>only to</code>). The signature of <code>main</code> is the whole program's grant.</p></div>
<div><span class="n">02</span><h3>Every run is evidence</h3><p>A run can be recorded, replayed offline and reproduced under its manifest; <code>--explain</code> narrates it in the words the program's author wrote.</p></div>
<div><span class="n">03</span><h3>Dependencies cannot widen what you allowed</h3><p>A package's effects are computed from its sources, never written by hand; an update that would let the program do more is refused, not noticed later.</p></div>
<div><span class="n">04</span><h3>The tooling an agent needs is built in</h3><p>The project map and the semantic diff are commands, the toolchain is an MCP server (<code>renyi mcp</code>), and every error carries a fix, so the agent's next attempt is right.</p></div>
</div>
</section>

<section class="gap">
<h2>The gap</h2>
<p>Between “the agent wrote it” and “I ran it” there is a review that nobody has time for.</p>
<p>Renyi closes that gap inside the language: the effects are types, the grant is a signature, the run is data, and the syntax exists so that a person can check all three in the time it takes to read a function header.</p>
</section>

<section class="install" id="install">
<div>
<h2>Install</h2>
<p class="note">Both installers download the release for the machine, verify its checksum and put <code>renyi</code> in place. With a Rust toolchain, <code>cargo install renyi</code> builds the same binary.</p>
</div>
<div>
<p class="label">Linux and macOS</p>
<pre><code>curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh</code></pre>
<p class="label">Windows, in PowerShell</p>
<pre><code>irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex</code></pre>
<p class="label">With Rust</p>
<pre><code>cargo install renyi</code></pre>
</div>
</section>

<section>
<h2>Read</h2>
<ul class="links">
<li><a href="cheatsheet.md">The cheat sheet <span>The whole language on one page</span></a></li>
<li><a href="reference.md">The reference <span>Every construct, its rules and diagnostics</span></a></li>
<li><a href="../examples/README.md">The examples <span>Programs in canonical layout</span></a></li>
<li><a href="../starter/README.md">The starter pack <span>A skill, the MCP configuration, five workflows</span></a></li>
<li><a href="ROADMAP.md">The roadmap <span>Every stage, its decisions and status</span></a></li>
<li><a href="https://github.com/renyi-lang/renyi">The repository <span>Apache-2.0</span></a></li>
</ul>
</section>
