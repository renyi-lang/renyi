<!--
The front page of the documentation site (decision AI2). Its wording comes
from docs/design/08-positioning.md (sections 2 and 6) and from the top of
README.md; tools/site.py gives it the layout of `main.home`.
-->

<section class="hero">
<h1>The scripting language of AI&nbsp;agents.</h1>
<p class="tagline">The agent writes it, you review it at a glance, and the program can only do what it declares.</p>
<p class="actions"><a class="button primary" href="#install">Install</a> <a class="button" href="cheatsheet.md">Cheat sheet</a> <a class="button" href="https://github.com/renyi-lang/renyi">GitHub</a></p>
</section>

<section class="sample">
<h2>What a program may do is in its signature.</h2>
<pre><code><span class="k">public function</span> active_adult_emails(path: Path)
<span class="sig">  <span class="k">returns</span> List <span class="k">of</span> Email</span><span class="sig">  <span class="k">or fails with</span> FileError <span class="k">or</span> JsonError</span><span class="sig needs">  <span class="k">needs</span> filesystem.read</span>  <span class="k">purpose:</span><span class="c"> Read users from a JSON file and return the e-mails of active users who are at least 18.</span>

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
<div class="points">
<div><span class="n">01</span><h3>Effects are capabilities</h3><p>Scoped, budgeted and guarded; the compiler refuses what is not declared.</p></div>
<div><span class="n">02</span><h3>Every run is evidence</h3><p>Recorded, replayed offline and reproduced under its manifest.</p></div>
<div><span class="n">03</span><h3>Dependencies cannot widen</h3><p>Effects computed from sources; an update that would widen them is refused.</p></div>
<div><span class="n">04</span><h3>Tooling built in</h3><p>Project map, semantic diff, an MCP server, a fix on every error.</p></div>
</div>
</section>

<section class="gap">
<p>Between “the agent wrote it” and “I ran it” there is a review that nobody has time for.</p>
<p>The effects are types, the grant is a signature, the run is data.</p>
</section>

<section class="install" id="install">
<h2>Install</h2>
<dl>
<dt>Linux, macOS</dt>
<dd><pre><code>curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh</code></pre></dd>
<dt>Windows PowerShell</dt>
<dd><pre><code>irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex</code></pre></dd>
<dt>With Cargo</dt>
<dd><pre><code>cargo install renyi</code></pre></dd>
</dl>
</section>

<section class="read">
<ul class="links">
<li><a href="cheatsheet.md">Cheat sheet</a></li>
<li><a href="reference.md">Reference</a></li>
<li><a href="../examples/README.md">Examples</a></li>
<li><a href="../starter/README.md">Starter pack</a></li>
<li><a href="ROADMAP.md">Roadmap</a></li>
<li><a href="https://github.com/renyi-lang/renyi">GitHub</a></li>
</ul>
</section>
