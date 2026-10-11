<!--
The front page of the documentation site (decision AI2). Its wording comes
from docs/design/08-positioning.md (sections 2 and 6) and from the top of
README.md; tools/site.py gives it the layout of `main.home` and puts the
terminal session of its SESSION in place of the empty `terminal` div. The
four small drawings are the diagrams of docs/how-it-works.md, reduced.
-->

<section class="hero">
<h1>The scripting language of AI&nbsp;agents.</h1>
<p class="tagline">The agent writes it, you review it at a glance, and the program can only do what it declares.</p>
<p class="actions"><a class="button primary" href="#install">Install</a> <a class="button" href="cheatsheet.md">Cheat sheet</a> <a class="button" href="https://github.com/renyi-lang/renyi">GitHub</a></p>
</section>

<section class="sample">
<h2>What a program may do is in its signature.</h2>
<pre><code><span class="k">public function</span> active_adult_emails(path: Path)
<span class="sig d1">  <span class="k">returns</span> List <span class="k">of</span> Email</span><span class="sig d2">  <span class="k">or fails with</span> FileError <span class="k">or</span> JsonError</span><span class="sig needs d3">  <span class="k">needs</span> filesystem.read</span>  <span class="k">purpose:</span><span class="c"> Read users from a JSON file and return the e-mails of active users who are at least 18.</span>

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
<div><span class="n">01</span>
<svg class="dg" viewBox="0 0 260 130" role="img" aria-labelledby="s1t s1d"><title id="s1t">The signature is the grant</title><desc id="s1d">Three signature lines: returns Config, or fails with FileError, needs filesystem.read scoped to data/. What is declared is admitted; the rest is refused.</desc>
<g class="p1"><rect class="tint" x="0" y="6" width="260" height="24"/><rect class="rail" x="0" y="6" width="2.5" height="24"/><text class="mono sm" x="12" y="22">returns Config</text>
<rect class="tint" x="0" y="32" width="260" height="24"/><rect class="rail" x="0" y="32" width="2.5" height="24"/><text class="mono sm" x="12" y="48">or fails with FileError</text>
<rect class="tint2" x="0" y="58" width="260" height="24"/><rect class="rail" x="0" y="58" width="2.5" height="24"/><text class="mono sm" x="12" y="74">needs <tspan class="acc">filesystem.read("data/")</tspan></text></g>
<g class="p2"><path class="ok" d="M3 108 L6.5 111.5 L13 104"/><text class="sm" x="20" y="112">declared: admitted</text>
<path class="no" d="M140 104 L148 112 M148 104 L140 112"/><text class="sm mut" x="155" y="112">the rest: refused</text></g>
</svg>
<h3>Effects are capabilities</h3><p>Scoped, budgeted and guarded; the compiler refuses what is not declared.</p></div>
<div><span class="n">02</span>
<svg class="dg" viewBox="0 0 260 130" role="img" aria-labelledby="s2t s2d"><title id="s2t">The run is evidence</title><desc id="s2d">renyi record writes run.json with its manifest; renyi run --replay and renyi reproduce answer every effect from it, offline.</desc>
<g class="p1"><rect class="box" x="0.5" y="8.5" width="75" height="30" rx="15"/><text class="mono sm" x="38" y="27" text-anchor="middle">record</text>
<rect class="box" x="92.5" y="8.5" width="75" height="30" rx="15"/><text class="mono sm" x="130" y="27" text-anchor="middle">replay</text>
<rect class="box" x="184.5" y="8.5" width="75" height="30" rx="15"/><text class="mono sm" x="222" y="27" text-anchor="middle">reproduce</text>
<path class="thin" d="M78 23.5 H87 M170 23.5 H179"/><path class="tip2" d="M86 20 L91 23.5 L86 27 Z M178 20 L183 23.5 L178 27 Z"/></g>
<g class="p2"><rect class="box" x="80.5" y="70.5" width="99" height="44" rx="6"/><text class="mono sm" x="130" y="89" text-anchor="middle">run.json</text><text class="sm mut" x="130" y="105" text-anchor="middle">+ manifest</text>
<path class="wire" d="M38 40 V92 H74 M130 70 V46 M180 92 H222 V46"/><path class="tip" d="M74 88 L80 92 L74 96 Z M126 46 L130 40 L134 46 Z M218 46 L222 40 L226 46 Z"/></g>
<g class="p3"><text class="sm mut" x="222" y="110" text-anchor="middle">offline</text></g>
</svg>
<h3>Every run is evidence</h3><p>Recorded, replayed offline and reproduced under its manifest.</p></div>
<div><span class="n">03</span>
<svg class="dg" viewBox="0 0 260 130" role="img" aria-labelledby="s3t s3d"><title id="s3t">Dependencies cannot widen</title><desc id="s3d">The locked version 1.0.0 needs console; the offered 1.1.0 adds network.http. renyi update refuses it unless --accept-effects is given and main declares the new capability.</desc>
<g class="p1"><rect class="box" x="0.5" y="4.5" width="109" height="62" rx="8"/><text class="cap" x="10" y="21">LOCKED 1.0.0</text><text class="mono sm" x="10" y="40">console</text>
<rect class="box" x="150.5" y="4.5" width="109" height="62" rx="8"/><text class="cap" x="160" y="21">OFFERED 1.1.0</text><text class="mono sm" x="160" y="40">console</text><text class="mono sm acc" x="160" y="56">+ network.http</text>
<path class="thin" d="M112 35.5 H143"/><path class="tip2" d="M142 32 L147 35.5 L142 39 Z"/></g>
<g class="p2"><path class="wire" d="M205 67 V76 H40 V84"/><path class="tip" d="M36 84 L40 90 L44 84 Z"/>
<text class="mono sm acc" x="0" y="106">renyi update</text><path class="no" d="M96 98 L104 106 M104 98 L96 106"/><text class="sm" x="112" y="106">refused: the effects widen</text></g>
<g class="p3"><text class="sm mut" x="0" y="126">unless <tspan class="mono">--accept-effects</tspan> and main declares it</text></g>
</svg>
<h3>Dependencies cannot widen</h3><p>Effects computed from sources; an update that would widen them is refused.</p></div>
<div><span class="n">04</span>
<svg class="dg" viewBox="0 0 260 130" role="img" aria-labelledby="s4t s4d"><title id="s4t">One toolchain</title><desc id="s4d">A source file goes through the front end, written in Rust and again in Renyi and held equal, to bytecode, which the VM runs on its interpreter, its template tier and Cranelift.</desc>
<g class="p1"><text class="cap" x="58" y="10">FRONT END</text><rect class="box" x="0.5" y="22.5" width="39" height="28" rx="8"/><text class="mono sm" x="20" y="40" text-anchor="middle">.ry</text>
<path class="thin" d="M42 36.5 H51"/><path class="tip2" d="M50 33 L55 36.5 L50 40 Z"/>
<rect class="frame" x="58.5" y="15.5" width="201" height="42" rx="9"/><rect class="box" x="66.5" y="22.5" width="77" height="28" rx="6"/><text class="sm" x="105" y="40" text-anchor="middle">Rust</text>
<text class="sm b acc" x="159" y="40" text-anchor="middle">=</text><rect class="box" x="174.5" y="22.5" width="77" height="28" rx="6"/><text class="sm" x="213" y="40" text-anchor="middle">Renyi</text></g>
<g class="p2"><path class="wire" d="M159 58 V68 H22 V80"/><path class="tip" d="M18 80 L22 86 L26 80 Z"/>
<rect class="box" x="0.5" y="86.5" width="43" height="28" rx="8"/><text class="mono sm" x="22" y="104" text-anchor="middle">.ryc</text></g>
<g class="p3"><text class="cap" x="260" y="76" text-anchor="end">THE VM</text><path class="thin" d="M46 100.5 H51"/><path class="tip2" d="M50 97 L55 100.5 L50 104 Z"/>
<rect class="frame" x="58.5" y="82.5" width="201" height="36" rx="9"/><rect class="box" x="63.5" y="87.5" width="63" height="26" rx="5"/><text class="xs" x="95" y="104" text-anchor="middle">interpreter</text>
<rect class="box" x="129.5" y="87.5" width="63" height="26" rx="5"/><text class="xs" x="161" y="104" text-anchor="middle">template</text>
<rect class="tint2" x="195.5" y="87.5" width="59" height="26" rx="5"/><text class="xs" x="225" y="104" text-anchor="middle">Cranelift</text></g>
</svg>
<h3>Tooling built in</h3><p>Project map, semantic diff, an MCP server, a fix on every error.</p></div>
</div>
<p class="more"><a href="how-it-works.md">How it works</a></p>
</section>

<section class="gap">
<p>Between “the agent wrote it” and “I ran it” there is a review that nobody has time for.</p>
<p>The effects are types, the grant is a signature, the run is data.</p>
<figure><div class="terminal"></div></figure>
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
