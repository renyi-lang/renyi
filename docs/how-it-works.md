<!--
How it works: the four points of docs/design/08-positioning.md (section 2),
each drawn as the mechanism that keeps it, with the terminal session of
tools/site.py (SESSION) in place of the empty `terminal` div. Every label is
true of the implementation as docs/reference.md describes it; the commands
are those of its appendix B. tools/site.py adds the group and the lead
around the title and gives the page the layout of `main.feature`.
-->

<h1>How it works</h1>

<nav class="chapters" aria-label="On this page"><ol>
<li><a href="#session">One session</a></li>
<li><a href="#grant"><span>01</span>The signature is the grant</a></li>
<li><a href="#evidence"><span>02</span>Every run is evidence</a></li>
<li><a href="#dependencies"><span>03</span>Dependencies cannot widen</a></li>
<li><a href="#toolchain"><span>04</span>One toolchain, written twice</a></li>
</ol></nav>

<section class="session">
<div class="prose">
<h2 id="session">One session</h2>
<p>The program below reads <code>todo.txt</code>, but its <code>main</code> declares only <code>console</code>. The checker refuses it and says what to write. After the fix it checks clean, <code>renyi record</code> runs it and writes <code>run.json</code>, and with the input file deleted <code>renyi run --replay</code> still answers every effect from the recording, narrated by <code>--explain</code>. Every line of output is what the binary printed.</p>
</div>
<pre><code>module todo
  purpose: Count the items of a to-do list.

import std.filesystem exposing Path, FileError
import std.console

public function main() or fails with FileError needs console
  purpose: Print how many items the list holds.

  let text be filesystem.read_text(Path("todo.txt")) otherwise fail
  console.print("{text.lines().length()} items")
end</code></pre>
<div class="terminal"></div>
</section>

<section class="mechanism">
<div class="prose">
<span class="num">01</span>
<h2 id="grant">The signature is the grant</h2>
<p>Every function says what it needs, and a need can be scoped to a path, a host or a variable, budgeted (<code>at most 60 per minute</code>) and guarded (<code>only to</code>). The checker refuses a body that uses a capability its head does not declare and names the fix; the declaration of <code>main</code> is the whole program’s grant. At run time every effect passes one boundary in the VM, which checks the actual path or host against the grant of the running call chain, so a read outside <code>data/</code> fails with <code>PermissionDenied</code>. <code>--deny</code>, <code>--allow-read</code> and their kin narrow the grant further from the command line, without touching the program.</p>
</div>
<figure>
<svg class="dg" viewBox="0 0 480 328" role="img" aria-labelledby="f1t f1d"><title id="f1t">The signature is the grant</title><desc id="f1d">A function head: public function load(path: Path), returns Config, or fails with FileError, needs filesystem.read("data/"). At compile time, renyi check refuses a body that uses a capability the head does not declare, with the code capability-missing. At run time, a read under data/ is admitted and a read of keys/id fails with PermissionDenied.</desc>
<g class="p1"><rect class="box" x="0.5" y="0.5" width="479" height="128" rx="12"/>
<text class="mono" x="20" y="28">public function load(path: Path)</text>
<rect class="tint" x="1" y="42" width="478" height="24"/><rect class="rail" x="1" y="42" width="3" height="24"/><text class="mono" x="36" y="58.5">returns Config</text><text class="small muted" x="460" y="58.5" text-anchor="end">what it gives</text>
<rect class="tint" x="1" y="66" width="478" height="24"/><rect class="rail" x="1" y="66" width="3" height="24"/><text class="mono" x="36" y="82.5">or fails with FileError</text><text class="small muted" x="460" y="82.5" text-anchor="end">how it fails</text>
<rect class="tint-strong" x="1" y="90" width="478" height="24"/><rect class="rail" x="1" y="90" width="3" height="24"/><text class="mono" x="36" y="106.5">needs <tspan class="accent">filesystem.read("data/")</tspan></text><text class="small strong accent" x="460" y="106.5" text-anchor="end">what it may do</text></g>
<g class="p2"><path class="flow" d="M240 129 V149 M124 168 V149 H356 V168"/><path class="head" d="M120 168 L124 174 L128 168 Z M352 168 L356 174 L360 168 Z"/><text class="label" x="250" y="144">THE GRANT</text></g>
<g class="p3"><rect class="frame" x="0.5" y="174.5" width="229" height="152" rx="12"/>
<text class="label" x="20" y="198">COMPILE TIME</text><text class="mono strong accent" x="20" y="222">renyi check</text>
<text class="small" x="20" y="248">A body that uses a capability</text><text class="small" x="20" y="264">the head does not declare:</text>
<path class="no" d="M20 282 L27 289 M27 282 L20 289"/><text class="mono" x="36" y="290">capability-missing</text><text class="small muted" x="20" y="310">refused, with the fix</text>
<rect class="frame" x="250.5" y="174.5" width="229" height="152" rx="12"/>
<text class="label" x="270" y="198">RUN TIME</text><text class="small muted" x="270" y="222">every effect, at one boundary</text>
<path class="ok" d="M270 244 L273.5 247.5 L280 241"/><text class="small" x="288" y="248">a read under <tspan class="mono">data/</tspan></text><text class="small muted" x="288" y="264">admitted</text>
<path class="no" d="M270 283 L277 290 M277 283 L270 290"/><text class="small" x="288" y="290">a read of <tspan class="mono">keys/id</tspan></text><text class="small muted" x="288" y="306">fails with <tspan class="mono">PermissionDenied</tspan></text></g>
</svg>
<figcaption>The runtime admits exactly what the signature declares.</figcaption>
</figure>
</section>

<section class="mechanism">
<div class="prose">
<span class="num">02</span>
<h2 id="evidence">Every run is evidence</h2>
<p><code>renyi record</code> runs <code>main</code> and writes every effect the run makes, with its result, and the run manifest in the header: the toolchain, the code hash, the dependencies with their versions and hashes, the grant, the arguments, the outcome and the output hash. <code>renyi run --replay</code> runs the program again with every effect answered from the recording, so nothing is written or sent. <code>renyi reproduce</code> replays a recording under its manifest and compares the outcome and the output byte for byte, refusing a program whose code hash is not the manifest’s. <code>--explain</code> narrates a run through the <code>purpose:</code> clauses it passes, and a <code>replays</code> test pins a function to a recording.</p>
</div>
<figure>
<svg class="dg" viewBox="0 0 480 300" role="img" aria-labelledby="f2t f2d"><title id="f2t">Every run is evidence</title><desc id="f2d">renyi record runs main and writes run.json: the manifest (toolchain, code hash, dependencies, grant, arguments, outcome, output hash) and every effect in order with its result. Offline, renyi run --replay answers every effect from the recording, and renyi reproduce replays it under the manifest and compares the outcome and the output.</desc>
<g class="p1"><rect class="box" x="0.5" y="0.5" width="230" height="72" rx="12"/>
<text class="mono strong accent" x="20" y="27">renyi record</text><text class="small muted" x="20" y="46">runs main and writes every</text><text class="small muted" x="20" y="61">effect with the manifest</text>
<rect class="box" x="260.5" y="0.5" width="219" height="299" rx="12"/>
<text class="mono strong" x="280" y="30">run.json</text><text class="label" x="280" y="56">MANIFEST</text>
<text class="small" x="280" y="77">toolchain</text><text class="small" x="280" y="94">code hash</text><text class="small" x="280" y="111">dependencies, hashes</text><text class="small" x="280" y="128">grant</text><text class="small" x="280" y="145">arguments</text><text class="small" x="280" y="162">outcome</text><text class="small" x="280" y="179">output hash</text>
<path class="seq" d="M280 198.5 H460"/><text class="label" x="280" y="222">CALLS</text><text class="small" x="280" y="242">every effect, in order,</text><text class="small" x="280" y="259">with its result</text></g>
<g class="p2"><path class="flow" d="M231 34 H254 M260 162 H225 M260 252 H225"/><path class="head" d="M254 30 L260 34 L254 38 Z M225 158 L219 162 L225 166 Z M225 248 L219 252 L225 256 Z"/>
<path class="seq" d="M115.5 73 V120 M115.5 197 V212"/><path class="head-seq" d="M111.5 120 L115.5 126 L119.5 120 Z M111.5 212 L115.5 218 L119.5 212 Z"/></g>
<g class="p3"><rect class="dash" x="0.5" y="92.5" width="230" height="207" rx="14"/><text class="label" x="20" y="115">OFFLINE</text>
<rect class="chip" x="12.5" y="126.5" width="206" height="70" rx="10"/>
<text class="mono strong accent" x="28" y="152">renyi run --replay</text><text class="small muted" x="28" y="171">answers every effect from</text><text class="small muted" x="28" y="186">the recording; nothing sent</text>
<rect class="chip" x="12.5" y="218.5" width="206" height="70" rx="10"/>
<text class="mono strong accent" x="28" y="244">renyi reproduce</text><text class="small muted" x="28" y="263">replays under the manifest,</text><text class="small muted" x="28" y="278">compares outcome and output</text></g>
</svg>
<figcaption>Same program, same inputs, same result.</figcaption>
</figure>
</section>

<section class="mechanism">
<div class="prose">
<span class="num">03</span>
<h2 id="dependencies">Dependencies cannot widen</h2>
<p>A package’s effect manifest is computed by the tool from its sources, not written by its author, and published with it. <code>renyi add</code> prints the effects a dependency brings; <code>renyi update</code> refuses a version whose effects widen unless <code>--accept-effects</code> is given, and even then takes it only when every <code>main</code> that reaches the package declares the new capability; <code>renyi audit</code> checks every locked dependency against every <code>main</code> that reaches it. On fetch every file is verified against its hash and the manifest recomputed, so a package cannot understate what it does.</p>
</div>
<figure>
<svg class="dg" viewBox="0 0 480 316" role="img" aria-labelledby="f3t f3d"><title id="f3t">Dependencies cannot widen</title><desc id="f3d">The effect manifest of greeting 1.1.0 is computed from its sources: console and network.http. The lockfile holds greeting 1.0.0 with console. renyi update compares the two and refuses the update because the effects widen. renyi update --accept-effects takes 1.1.0 only when every main that reaches it declares network.http.</desc>
<g class="p1"><rect class="box" x="0.5" y="0.5" width="165" height="100" rx="12"/>
<text class="label" x="20" y="24">PACKAGE SOURCES</text><text class="mono strong" x="20" y="50">greeting 1.1.0</text><text class="small muted" x="20" y="72">its .ry files, each</text><text class="small muted" x="20" y="88">with its content hash</text>
<path class="flow" d="M166 50 H302"/><path class="head" d="M302 46 L308 50 L302 54 Z"/><text class="small strong" x="236" y="41" text-anchor="middle">computed</text><text class="small muted" x="236" y="69" text-anchor="middle">not written</text>
<rect class="box" x="308.5" y="0.5" width="171" height="100" rx="12"/>
<text class="label" x="328" y="24">EFFECT MANIFEST</text><text class="mono" x="328" y="50">console</text><text class="mono accent" x="328" y="68">network.http</text><text class="small muted" x="328" y="88">new in 1.1.0</text></g>
<g class="p2"><rect class="box" x="0.5" y="128.5" width="165" height="80" rx="12"/>
<text class="label" x="20" y="152">LOCKFILE</text><text class="mono strong" x="20" y="176">greeting 1.0.0</text><text class="mono" x="20" y="195">console</text>
<path class="flow" d="M166 168 H302 M394 101 V122"/><path class="head" d="M302 164 L308 168 L302 172 Z M390 122 L394 128 L398 122 Z"/>
<rect class="hot" x="308.5" y="128.5" width="171" height="80" rx="12"/>
<text class="mono strong accent" x="328" y="154">renyi update</text><path class="no" d="M328 169 L335 176 M335 169 L328 176"/><text class="small strong" x="344" y="176">refused:</text><text class="small" x="344" y="193">the effects widen</text></g>
<g class="p3"><rect class="dash" x="0.5" y="236.5" width="479" height="78" rx="12"/>
<text class="mono strong" x="20" y="262">renyi update --accept-effects</text><text class="small muted" x="20" y="283">takes 1.1.0 only when every main that reaches it</text><text class="small muted" x="20" y="300">declares <tspan class="mono">network.http</tspan></text></g>
</svg>
<figcaption>On fetch, every file is checked against its hash and the manifest is recomputed from the sources.</figcaption>
</figure>
</section>

<section class="mechanism">
<div class="prose">
<span class="num">04</span>
<h2 id="toolchain">One toolchain, written twice</h2>
<p>One binary, <code>renyi</code>, written in Rust. The front end lexes, parses, checks and emits bytecode, and it exists twice: in Rust, and written in Renyi under <code>compiler/</code> and run by the VM. Tests hold the two equal byte for byte (the syntax tree, the diagnostics and the bytecode file) on every program of the corpus, the conformance suite and the compiler itself. The VM runs a function on its template tier from the first call and moves hot code to Cranelift; <code>--interpret</code> keeps a run on the interpreter, as <code>--explain</code> and <code>--profile</code> do. <code>renyi build</code> compiles every function ahead into an image that <code>run</code>, <code>record</code>, <code>test</code> and <code>reproduce</code> load without compiling anything.</p>
</div>
<figure>
<svg class="dg" viewBox="0 0 480 488" role="img" aria-labelledby="f4t f4d"><title id="f4t">One toolchain, written twice</title><desc id="f4d">A source file goes through the front end: lexer, parser, checker, emitter, once in Rust under crates/ and once in Renyi under compiler/, run by the VM, held equal byte for byte by tests. The bytecode, a .ryc file, runs on the VM: the interpreter (--interpret), the template tier from the first call, the Cranelift tier when the code is hot. renyi build writes an image, .ryi, with every function as machine code.</desc>
<g class="p1"><rect class="chip" x="176.5" y="0.5" width="127" height="32" rx="16"/><text class="mono" x="240" y="21" text-anchor="middle">program.ry</text>
<path class="seq" d="M240 33 V50"/><path class="head-seq" d="M236 50 L240 56 L244 50 Z"/>
<rect class="frame" x="0.5" y="56.5" width="479" height="190" rx="14"/><text class="label" x="20" y="80">THE FRONT END</text>
<text class="small strong" x="20" y="104">in Rust <tspan class="muted" font-weight="400">· crates/</tspan></text>
<rect class="chip" x="19.5" y="114.5" width="100" height="30" rx="8"/><text class="small" x="69.5" y="134" text-anchor="middle">lexer</text>
<rect class="chip" x="133.5" y="114.5" width="100" height="30" rx="8"/><text class="small" x="183.5" y="134" text-anchor="middle">parser</text>
<rect class="chip" x="247.5" y="114.5" width="100" height="30" rx="8"/><text class="small" x="297.5" y="134" text-anchor="middle">checker</text>
<rect class="chip" x="361.5" y="114.5" width="100" height="30" rx="8"/><text class="small" x="411.5" y="134" text-anchor="middle">emitter</text>
<text class="small strong accent" x="240" y="170" text-anchor="middle">= held equal, byte for byte, by tests</text>
<text class="small strong" x="20" y="196">in Renyi <tspan class="muted" font-weight="400">· compiler/, run by the VM</tspan></text>
<rect class="chip" x="19.5" y="206.5" width="100" height="30" rx="8"/><text class="small" x="69.5" y="226" text-anchor="middle">lexer</text>
<rect class="chip" x="133.5" y="206.5" width="100" height="30" rx="8"/><text class="small" x="183.5" y="226" text-anchor="middle">parser</text>
<rect class="chip" x="247.5" y="206.5" width="100" height="30" rx="8"/><text class="small" x="297.5" y="226" text-anchor="middle">checker</text>
<rect class="chip" x="361.5" y="206.5" width="100" height="30" rx="8"/><text class="small" x="411.5" y="226" text-anchor="middle">emitter</text>
<path class="seq" d="M121 129.5 H127 M235 129.5 H241 M349 129.5 H355 M121 221.5 H127 M235 221.5 H241 M349 221.5 H355"/>
<path class="head-seq" d="M127 126 L132 129.5 L127 133 Z M241 126 L246 129.5 L241 133 Z M355 126 L360 129.5 L355 133 Z M127 218 L132 221.5 L127 225 Z M241 218 L246 221.5 L241 225 Z M355 218 L360 221.5 L355 225 Z"/></g>
<g class="p2"><path class="flow" d="M240 247 V264"/><path class="head" d="M236 264 L240 270 L244 264 Z"/>
<rect class="chip" x="160.5" y="270.5" width="159" height="32" rx="16"/><text class="small" x="240" y="291" text-anchor="middle">bytecode <tspan class="mono">.ryc</tspan></text>
<path class="flow" d="M240 303 V320"/><path class="head" d="M236 320 L240 326 L244 320 Z"/></g>
<g class="p3"><rect class="frame" x="0.5" y="326.5" width="479" height="100" rx="14"/><text class="label" x="20" y="350">THE VM</text>
<rect class="chip" x="21.5" y="362.5" width="138" height="50" rx="10"/><text class="small strong" x="90.5" y="383" text-anchor="middle">interpreter</text><text class="mono muted" x="90.5" y="401" text-anchor="middle">--interpret</text>
<rect class="chip" x="171.5" y="362.5" width="138" height="50" rx="10"/><text class="small strong" x="240.5" y="383" text-anchor="middle">template tier</text><text class="small muted" x="240.5" y="401" text-anchor="middle">from the first call</text>
<rect class="hot" x="321.5" y="362.5" width="138" height="50" rx="10"/><text class="small strong" x="390.5" y="383" text-anchor="middle">Cranelift tier</text><text class="small muted" x="390.5" y="401" text-anchor="middle">when the code is hot</text>
<path class="seq" d="M311 387.5 H315"/><path class="head-seq" d="M315 384 L320 387.5 L315 391 Z"/>
<rect class="dash" x="0.5" y="446.5" width="479" height="40" rx="12"/><text class="small" x="20" y="471"><tspan class="mono strong accent">renyi build</tspan> writes an image, <tspan class="mono">.ryi</tspan>: every function as machine code</text>
<path class="seq" d="M390.5 446 V420"/><path class="head-seq" d="M386.5 420 L390.5 414 L394.5 420 Z"/></g>
</svg>
<figcaption>Tests hold the two front ends equal, byte for byte.</figcaption>
</figure>
</section>
