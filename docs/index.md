<!--
The front page of the documentation site (decision AI2). Its wording comes
from docs/design/08-positioning.md (sections 2, 3 and 6) and from the top of
README.md; tools/site.py gives it the layout of `main.home`, highlights the
sample and the commands, and puts the terminal session of its SESSION in
place of the empty `terminal` div. The four small drawings are the diagrams
of docs/how-it-works.md, reduced.
-->

<section class="hero">
<div class="in">
<div class="pitch">
<h1>The scripting language of AI&nbsp;agents.</h1>
<p class="tagline">The agent writes it, you review it at a glance, and the program can only do what it declares.</p>
<p class="actions"><a class="button primary" href="#install">Install</a> <a class="button" href="cheatsheet.md">Cheat sheet</a> <a class="button" href="https://github.com/renyi-lang/renyi">GitHub</a></p>
</div>
<figure class="sample">
<h2>What a program may do is in its signature.</h2>
<div class="card">
<pre><code>public function active_adult_emails(path: Path)
  returns List of Email
  or fails with FileError or JsonError
  needs filesystem.read
  purpose: Read users from a JSON file and return the e-mails of active users who are at least 18.

  let text be filesystem.read_text(path) otherwise fail
  let users: List of User be json.parse(text) otherwise fail
  let emails be
    for each user in users
    where user.is_active and user.age is at least 18
    sorted by user.name
    collect user.email
  return emails
end</code></pre>
<ol class="notes"><li><span class="sr">returns: </span>what it gives</li><li><span class="sr">or fails with: </span>how it fails</li><li><span class="sr">needs: </span>what it may do</li></ol>
</div>
</figure>
</div>
</section>

<section class="promises">
<div class="in">
<h2>Four promises the implementation keeps today.</h2>
<div class="points">
<div class="point">
<div><span class="num">01</span><h3>Effects are capabilities</h3><p>Scoped, budgeted and guarded; the compiler refuses what is not declared.</p></div>
<svg class="dg mini" viewBox="0 0 260 132" role="img" aria-labelledby="s1t s1d"><title id="s1t">The signature is the grant</title><desc id="s1d">A function head with three lines: returns Config, or fails with FileError, needs filesystem.read scoped to data/. What is declared is admitted; the rest is refused.</desc>
<g class="p1"><rect class="chip" x="0.5" y="0.5" width="259" height="95" rx="10"/><text class="mono" x="12" y="19">public function load(path: Path)</text>
<rect class="tint" x="1" y="28" width="258" height="18"/><rect class="rail" x="1" y="28" width="2.5" height="18"/><text class="mono" x="20" y="41">returns Config</text>
<rect class="tint" x="1" y="46" width="258" height="18"/><rect class="rail" x="1" y="46" width="2.5" height="18"/><text class="mono" x="20" y="59">or fails with FileError</text>
<rect class="tint-strong" x="1" y="64" width="258" height="18"/><rect class="rail" x="1" y="64" width="2.5" height="18"/><text class="mono" x="20" y="77">needs <tspan class="accent">filesystem.read("data/")</tspan></text></g>
<g class="p2"><path class="ok" d="M2 119 L5.5 122.5 L12 116"/><text x="19" y="123">declared: admitted</text>
<path class="no" d="M140 116 L147 123 M147 116 L140 123"/><text class="muted" x="155" y="123">the rest: refused</text></g>
</svg>
</div>
<div class="point">
<div><span class="num">02</span><h3>Every run is evidence</h3><p>Recorded, replayed offline and reproduced under its manifest.</p></div>
<svg class="dg mini" viewBox="0 0 260 132" role="img" aria-labelledby="s2t s2d"><title id="s2t">The run is evidence</title><desc id="s2d">renyi record writes run.json with its manifest; renyi run --replay and renyi reproduce answer every effect from it, offline.</desc>
<g class="p1"><rect class="chip" x="0.5" y="6.5" width="76" height="28" rx="14"/><text class="mono" x="38.5" y="24.5" text-anchor="middle">record</text>
<rect class="chip" x="92.5" y="6.5" width="76" height="28" rx="14"/><text class="mono" x="130.5" y="24.5" text-anchor="middle">replay</text>
<rect class="chip" x="183.5" y="6.5" width="76" height="28" rx="14"/><text class="mono" x="221.5" y="24.5" text-anchor="middle">reproduce</text>
<path class="seq" d="M79 20.5 H86 M171 20.5 H177"/><path class="head-seq" d="M86 17 L91 20.5 L86 24 Z M177 17 L182 20.5 L177 24 Z"/></g>
<g class="p2"><rect class="box" x="84.5" y="76.5" width="92" height="44" rx="8"/><text class="mono" x="130.5" y="95" text-anchor="middle">run.json</text><text class="tiny muted" x="130.5" y="111" text-anchor="middle">+ manifest</text>
<path class="flow" d="M38.5 36 V98.5 H78 M130.5 76 V42 M177 98.5 H221.5 V42"/><path class="head" d="M78 94.5 L84 98.5 L78 102.5 Z M126.5 42 L130.5 36 L134.5 42 Z M217.5 42 L221.5 36 L225.5 42 Z"/></g>
<g class="p3"><text class="tiny muted" x="221.5" y="124" text-anchor="middle">offline</text></g>
</svg>
</div>
<div class="point">
<div><span class="num">03</span><h3>Dependencies cannot widen</h3><p>Effects computed from sources; an update that would widen them is refused.</p></div>
<svg class="dg mini" viewBox="0 0 260 132" role="img" aria-labelledby="s3t s3d"><title id="s3t">Dependencies cannot widen</title><desc id="s3d">The locked version 1.0.0 needs console; the offered 1.1.0 adds network.http. renyi update refuses it unless --accept-effects is given and main declares the new capability.</desc>
<g class="p1"><rect class="chip" x="0.5" y="0.5" width="98" height="60" rx="10"/><text class="label" x="11" y="19">LOCKED 1.0.0</text><text class="mono" x="11" y="40">console</text>
<rect class="chip" x="141.5" y="0.5" width="118" height="60" rx="10"/><text class="label" x="152" y="19">OFFERED 1.1.0</text><text class="mono" x="152" y="36">console</text><text class="mono accent" x="152" y="51">+ network.http</text>
<path class="seq" d="M102 30.5 H134"/><path class="head-seq" d="M134 27 L139 30.5 L134 34 Z"/></g>
<g class="p2"><path class="flow" d="M200.5 61 V70.5 H38.5 V79"/><path class="head" d="M34.5 79 L38.5 85 L42.5 79 Z"/>
<text class="mono accent" x="0" y="102">renyi update</text><path class="no" d="M95 95 L102 102 M102 95 L95 102"/><text x="109" y="102">refused: the effects widen</text></g>
<g class="p3"><text class="tiny muted" x="0" y="124">unless <tspan class="mono">--accept-effects</tspan> and main declares it</text></g>
</svg>
</div>
<div class="point">
<div><span class="num">04</span><h3>Tooling built in</h3><p>Project map, semantic diff, an MCP server, a fix on every error.</p></div>
<svg class="dg mini" viewBox="0 0 260 132" role="img" aria-labelledby="s4t s4d"><title id="s4t">One toolchain</title><desc id="s4d">A source file goes through the front end, written in Rust and again in Renyi and held equal, to bytecode, which the VM runs on its interpreter, its template tier and Cranelift.</desc>
<g class="p1"><text class="label" x="56" y="10">FRONT END</text><rect class="chip" x="0.5" y="22.5" width="38" height="28" rx="8"/><text class="mono" x="19.5" y="40.5" text-anchor="middle">.ry</text>
<path class="seq" d="M41 36.5 H49"/><path class="head-seq" d="M49 33 L54 36.5 L49 40 Z"/>
<rect class="frame" x="56.5" y="16.5" width="203" height="40" rx="10"/><rect class="chip" x="63.5" y="22.5" width="78" height="28" rx="7"/><text x="102.5" y="40.5" text-anchor="middle">Rust</text>
<text class="strong accent" x="158" y="40.5" text-anchor="middle">=</text><rect class="chip" x="174.5" y="22.5" width="78" height="28" rx="7"/><text x="213.5" y="40.5" text-anchor="middle">Renyi</text></g>
<g class="p2"><path class="flow" d="M158 57 V67.5 H21.5 V79"/><path class="head" d="M17.5 79 L21.5 85 L25.5 79 Z"/>
<rect class="chip" x="0.5" y="85.5" width="42" height="28" rx="8"/><text class="mono" x="21.5" y="103.5" text-anchor="middle">.ryc</text></g>
<g class="p3"><text class="label" x="259.5" y="74" text-anchor="end">THE VM</text><path class="seq" d="M45 99.5 H49"/><path class="head-seq" d="M49 96 L54 99.5 L49 103 Z"/>
<rect class="frame" x="56.5" y="79.5" width="203" height="40" rx="10"/><rect class="chip" x="61.5" y="85.5" width="63" height="28" rx="6"/><text class="tiny" x="93" y="103" text-anchor="middle">interpreter</text>
<rect class="chip" x="127.5" y="85.5" width="63" height="28" rx="6"/><text class="tiny" x="159" y="103" text-anchor="middle">template</text>
<rect class="hot" x="193.5" y="85.5" width="61" height="28" rx="6"/><text class="tiny" x="224" y="103" text-anchor="middle">Cranelift</text></g>
</svg>
</div>
</div>
<p class="more"><a href="how-it-works.md">How it works</a></p>
</div>
</section>

<section class="band invert">
<div class="in">
<div>
<h2>Between “the agent wrote it” and “I ran it” there is a review that nobody has time for.</h2>
<p class="then">The effects are types, the grant is a signature, the run is data.</p>
</div>
<div class="terminal"></div>
</div>
</section>

<section class="install" id="install">
<div class="in">
<div>
<h2>Install</h2>
<p>Every release carries binaries for Linux, macOS and Windows; both installers verify the archive’s checksum.</p>
<p>An agent starts from the <a href="../starter/README.md">starter pack</a>: a skill file, the MCP configuration and five workflows that do real work.</p>
</div>
<dl class="commands">
<div><dt>Linux, macOS</dt><dd><pre><code class="language-sh">curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh</code></pre></dd></div>
<div><dt>Windows PowerShell</dt><dd><pre><code class="language-powershell">irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex</code></pre></dd></div>
<div><dt>With Cargo</dt><dd><pre><code class="language-sh">cargo install renyi</code></pre></dd></div>
</dl>
</div>
</section>

<section class="read">
<div class="in">
<h2 class="sr">Documentation</h2>
<ul class="links">
<li><a href="cheatsheet.md"><strong>Cheat sheet</strong><span>The whole language on one page: every construct once.</span></a></li>
<li><a href="reference.md"><strong>Reference</strong><span>Each construct with its grammar, its meaning, its rules and its behaviour at run time.</span></a></li>
<li><a href="../examples/README.md"><strong>Examples</strong><span>Complete programs, one per file, that fixed the surface of the language.</span></a></li>
<li><a href="../starter/README.md"><strong>Starter pack</strong><span>What an agent needs in its first hour: the skill, the MCP configuration, five workflows.</span></a></li>
<li><a href="ROADMAP.md"><strong>Roadmap</strong><span>Every stage of the project, in order and by track, and what comes next.</span></a></li>
<li><a href="https://github.com/renyi-lang/renyi"><strong>GitHub</strong><span>The repository, under the Apache-2.0 license.</span></a></li>
</ul>
</div>
</section>
