<!--
How it works: the four points of docs/design/08-positioning.md (section 2),
each drawn as the mechanism that keeps it, with the terminal session of
tools/site.py (SESSION) in place of the empty `terminal` div. Every label is
true of the implementation as docs/reference.md describes it; the commands
are those of its appendix B. tools/site.py adds the lead under the title.
-->

<h1>How it works</h1>

<section class="demo">
<h2 id="session">One session</h2>
<p>The program below reads <code>todo.txt</code>, but its <code>main</code> declares only <code>console</code>. The checker refuses it and says what to write. After the fix it checks clean, <code>renyi record</code> runs it and writes <code>run.json</code>, and with the input file deleted <code>renyi run --replay</code> still answers every effect from the recording, narrated by <code>--explain</code>. Every line of output is what the binary printed.</p>
<pre><code>module todo
  purpose: Count the items of a to-do list.

import std.filesystem exposing Path, FileError
import std.console

public function main() or fails with FileError needs console
  purpose: Print how many items the list holds.

  let text be filesystem.read_text(Path("todo.txt")) otherwise fail
  console.print("{text.lines().length()} items")
end</code></pre>
<figure><div class="terminal"></div></figure>
</section>

<section>
<h2 id="grant">01 · The signature is the grant</h2>
<p>Every function says what it needs, and a need can be scoped to a path, a host or a variable, budgeted (<code>at most 60 per minute</code>) and guarded (<code>only to</code>). The checker refuses a body that uses a capability its head does not declare and names the fix; the declaration of <code>main</code> is the whole program’s grant. At run time every effect passes one boundary in the VM, which checks the actual path or host against the grant of the running call chain, so a read outside <code>data/</code> fails with <code>PermissionDenied</code>. <code>--deny</code>, <code>--allow-read</code> and their kin narrow the grant further from the command line, without touching the program.</p>
<figure>
<svg class="dg full" viewBox="0 0 440 340" role="img" aria-labelledby="f1t f1d"><title id="f1t">The signature is the grant</title><desc id="f1d">A function head: public function load(path: Path), returns Config, or fails with FileError, needs filesystem.read("data/"). At compile time, renyi check refuses a body that uses a capability the head does not declare, with the code capability-missing. At run time, a read under data/ is admitted and a read of keys/id fails with PermissionDenied. The runtime admits exactly what the signature declares.</desc>
<g class="p1"><rect class="box" x="0.5" y="0.5" width="439" height="117" rx="10"/>
<text class="mono" x="16" y="25">public function load(path: Path)</text>
<rect class="tint" x="1" y="37" width="438" height="22"/><rect class="rail" x="1" y="37" width="3" height="22"/><text class="mono" x="30" y="52">returns Config</text><text class="sm mut" x="424" y="52" text-anchor="end">what it gives</text>
<rect class="tint" x="1" y="59" width="438" height="22"/><rect class="rail" x="1" y="59" width="3" height="22"/><text class="mono" x="30" y="74">or fails with FileError</text><text class="sm mut" x="424" y="74" text-anchor="end">how it fails</text>
<rect class="tint2" x="1" y="81" width="438" height="22"/><rect class="rail" x="1" y="81" width="3" height="22"/><text class="mono" x="30" y="96">needs <tspan class="acc">filesystem.read("data/")</tspan></text><text class="sm b acc" x="424" y="96" text-anchor="end">what it may do</text></g>
<g class="p2"><path class="wire" d="M220 118 V138 M105 160 V138 H335 V160"/><path class="tip" d="M101 160 L105 166 L109 160 Z M331 160 L335 166 L339 160 Z"/><text class="cap" x="228" y="133">THE GRANT</text></g>
<g class="p3"><rect class="frame" x="0.5" y="168.5" width="209" height="138" rx="10"/>
<text class="cap" x="16" y="190">COMPILE TIME</text><text class="mono b acc" x="16" y="212">renyi check</text>
<text class="sm" x="16" y="238">A body that uses a capability</text><text class="sm" x="16" y="254">the head does not declare:</text>
<path class="no" d="M16 274 L24 282 M24 274 L16 282"/><text class="mono sm" x="32" y="282">capability-missing</text><text class="sm mut" x="16" y="298">refused, with the fix</text>
<rect class="frame" x="230.5" y="168.5" width="209" height="138" rx="10"/>
<text class="cap" x="246" y="190">RUN TIME</text><text class="sm mut" x="246" y="212">every effect, at one boundary</text>
<path class="ok" d="M244 236 L247.5 239.5 L254 232"/><text class="sm" x="260" y="240">a read under <tspan class="mono">data/</tspan></text><text class="sm mut" x="260" y="255">admitted</text>
<path class="no" d="M245 270 L253 278 M253 270 L245 278"/><text class="sm" x="260" y="278">a read of <tspan class="mono">keys/id</tspan></text><text class="sm mut" x="260" y="293">fails with <tspan class="mono">PermissionDenied</tspan></text>
<text class="sm mut" x="220" y="332" text-anchor="middle">The runtime admits exactly what the signature declares.</text></g>
</svg>
</figure>
</section>

<section>
<h2 id="evidence">02 · Every run is evidence</h2>
<p><code>renyi record</code> runs <code>main</code> and writes every effect the run makes, with its result, and the run manifest in the header: the toolchain, the code hash, the dependencies with their versions and hashes, the grant, the arguments, the outcome and the output hash. <code>renyi run --replay</code> runs the program again with every effect answered from the recording, so nothing is written or sent. <code>renyi reproduce</code> replays a recording under its manifest and compares the outcome and the output byte for byte, refusing a program whose code hash is not the manifest’s. <code>--explain</code> narrates a run through the <code>purpose:</code> clauses it passes, and a <code>replays</code> test pins a function to a recording.</p>
<figure>
<svg class="dg full" viewBox="0 0 440 340" role="img" aria-labelledby="f2t f2d"><title id="f2t">Every run is evidence</title><desc id="f2d">renyi record runs main and writes run.json: the manifest (toolchain, code hash, dependencies, grant, arguments, outcome, output hash) and every effect in order with its result. Offline, renyi run --replay answers every effect from the recording, and renyi reproduce replays it under the manifest and compares the outcome and the output. Same program, same inputs, same result.</desc>
<g class="p1"><rect class="box" x="8.5" y="0.5" width="200" height="64" rx="10"/>
<text class="mono b acc" x="22" y="24">renyi record</text><text class="sm mut" x="22" y="42">runs main and writes every</text><text class="sm mut" x="22" y="56">effect with the manifest</text>
<rect class="box" x="244.5" y="16.5" width="195" height="260" rx="8"/>
<text class="mono b" x="260" y="42">run.json</text><text class="cap" x="260" y="66">MANIFEST</text>
<text class="sm" x="260" y="84">toolchain</text><text class="sm" x="260" y="101">code hash</text><text class="sm" x="260" y="118">dependencies, hashes</text><text class="sm" x="260" y="135">grant</text><text class="sm" x="260" y="152">arguments</text><text class="sm" x="260" y="169">outcome</text><text class="sm" x="260" y="186">output hash</text>
<path class="thin" d="M260 202.5 H424"/><text class="cap" x="260" y="222">CALLS</text><text class="sm" x="260" y="240">every effect, in order,</text><text class="sm" x="260" y="257">with its result</text></g>
<g class="p2"><path class="wire" d="M209 32 H238 M244 150 H214 M244 268 H214"/><path class="tip" d="M238 28 L244 32 L238 36 Z M214 146 L208 150 L214 154 Z M214 264 L208 268 L214 272 Z"/>
<path class="thin" d="M108 65 V84 M108 183 V230"/><path class="tip2" d="M104 230 L108 236 L112 230 Z"/></g>
<g class="p3"><rect class="dash" x="0.5" y="90.5" width="217" height="219" rx="12"/><text class="cap" x="16" y="108">OFFLINE</text>
<rect class="box" x="8.5" y="118.5" width="200" height="64" rx="10"/>
<text class="mono b acc" x="22" y="142">renyi run --replay</text><text class="sm mut" x="22" y="160">answers every effect from</text><text class="sm mut" x="22" y="174">the recording; nothing sent</text>
<rect class="box" x="8.5" y="236.5" width="200" height="64" rx="10"/>
<text class="mono b acc" x="22" y="260">renyi reproduce</text><text class="sm mut" x="22" y="278">replays under the manifest,</text><text class="sm mut" x="22" y="292">compares outcome and output</text>
<text class="sm mut" x="220" y="333" text-anchor="middle">Same program, same inputs, same result.</text></g>
</svg>
</figure>
</section>

<section>
<h2 id="dependencies">03 · Dependencies cannot widen</h2>
<p>A package’s effect manifest is computed by the tool from its sources, not written by its author, and published with it. <code>renyi add</code> prints the effects a dependency brings; <code>renyi update</code> refuses a version whose effects widen unless <code>--accept-effects</code> is given, and even then takes it only when every <code>main</code> that reaches the package declares the new capability; <code>renyi audit</code> checks every locked dependency against every <code>main</code> that reaches it. On fetch every file is verified against its hash and the manifest recomputed, so a package cannot understate what it does.</p>
<figure>
<svg class="dg full" viewBox="0 0 440 350" role="img" aria-labelledby="f4t f4d"><title id="f4t">Dependencies cannot widen</title><desc id="f4d">The effect manifest of greeting 1.1.0 is computed from its sources: console and network.http. The lockfile holds greeting 1.0.0 with console. renyi update compares the two and refuses the update because the effects widen. renyi update --accept-effects takes 1.1.0 only when every main that reaches it declares network.http. On fetch, every file is checked against its hash and the manifest is recomputed.</desc>
<g class="p1"><rect class="box" x="0.5" y="0.5" width="149" height="96" rx="10"/>
<text class="cap" x="16" y="22">PACKAGE SOURCES</text><text class="mono b" x="16" y="46">greeting 1.1.0</text><text class="sm mut" x="16" y="66">its .ry files, each</text><text class="sm mut" x="16" y="82">with its content hash</text>
<path class="wire" d="M150 48 H264"/><path class="tip" d="M264 44 L270 48 L264 52 Z"/><text class="sm b" x="208" y="40" text-anchor="middle">computed</text><text class="sm mut" x="208" y="66" text-anchor="middle">not written</text>
<rect class="box" x="270.5" y="0.5" width="169" height="96" rx="10"/>
<text class="cap" x="286" y="22">EFFECT MANIFEST</text><text class="mono" x="286" y="46">console</text><text class="mono acc" x="286" y="64">network.http</text><text class="sm mut" x="286" y="84">new in 1.1.0</text></g>
<g class="p2"><rect class="box" x="0.5" y="124.5" width="149" height="72" rx="10"/>
<text class="cap" x="16" y="146">LOCKFILE</text><text class="mono b" x="16" y="168">greeting 1.0.0</text><text class="mono" x="16" y="186">console</text>
<path class="wire" d="M150 160 H264 M355 97 V118"/><path class="tip" d="M264 156 L270 160 L264 164 Z M351 118 L355 124 L359 118 Z"/>
<rect class="tint" x="270.5" y="124.5" width="169" height="72" rx="10"/>
<text class="mono b acc" x="286" y="148">renyi update</text><path class="no" d="M286 166 L294 174 M294 166 L286 174"/><text class="sm b" x="302" y="174">refused:</text><text class="sm" x="302" y="189">the effects widen</text></g>
<g class="p3"><rect class="dash" x="0.5" y="226.5" width="439" height="72" rx="10"/>
<text class="mono b" x="16" y="252">renyi update --accept-effects</text><text class="sm mut" x="16" y="272">takes 1.1.0 only when every main that reaches it</text><text class="sm mut" x="16" y="288">declares <tspan class="mono">network.http</tspan></text>
<text class="sm mut" x="220" y="324" text-anchor="middle">On fetch, every file is checked against its hash</text><text class="sm mut" x="220" y="341" text-anchor="middle">and the manifest is recomputed from the sources.</text></g>
</svg>
</figure>
</section>

<section>
<h2 id="toolchain">04 · One toolchain, written twice</h2>
<p>One binary, <code>renyi</code>, written in Rust. The front end lexes, parses, checks and emits bytecode, and it exists twice: in Rust, and written in Renyi under <code>compiler/</code> and run by the VM. Tests hold the two equal byte for byte (the syntax tree, the diagnostics and the bytecode file) on every program of the corpus, the conformance suite and the compiler itself. The VM runs a function on its template tier from the first call and moves hot code to Cranelift; <code>--interpret</code> keeps a run on the interpreter, as <code>--explain</code> and <code>--profile</code> do. <code>renyi build</code> compiles every function ahead into an image that <code>run</code>, <code>record</code>, <code>test</code> and <code>reproduce</code> load without compiling anything.</p>
<figure>
<svg class="dg full" viewBox="0 0 440 470" role="img" aria-labelledby="f3t f3d"><title id="f3t">One toolchain, written twice</title><desc id="f3d">A source file goes through the front end: lexer, parser, checker, emitter, once in Rust under crates/ and once in Renyi under compiler/, run by the VM, held equal byte for byte by tests. The bytecode, a .ryc file, runs on the VM: the interpreter (--interpret), the template tier from the first call, the Cranelift tier when the code is hot. renyi build writes an image, .ryi, with every function as machine code.</desc>
<g class="p1"><rect class="box" x="160.5" y="0.5" width="119" height="32" rx="16"/><text class="mono sm" x="220" y="21" text-anchor="middle">program.ry</text>
<path class="thin" d="M220 33 V50"/><path class="tip2" d="M216 50 L220 56 L224 50 Z"/>
<rect class="frame" x="0.5" y="56.5" width="439" height="180" rx="12"/><text class="cap" x="16" y="78">THE FRONT END</text>
<text class="sm b" x="16" y="100">in Rust <tspan class="mut" font-weight="400">· crates/</tspan></text>
<rect class="box" x="16.5" y="108.5" width="89" height="28" rx="6"/><text class="sm" x="61" y="127" text-anchor="middle">lexer</text>
<rect class="box" x="122.5" y="108.5" width="89" height="28" rx="6"/><text class="sm" x="167" y="127" text-anchor="middle">parser</text>
<rect class="box" x="228.5" y="108.5" width="89" height="28" rx="6"/><text class="sm" x="273" y="127" text-anchor="middle">checker</text>
<rect class="box" x="334.5" y="108.5" width="89" height="28" rx="6"/><text class="sm" x="379" y="127" text-anchor="middle">emitter</text>
<text class="sm b acc" x="220" y="160" text-anchor="middle">= held equal, byte for byte, by tests</text>
<text class="sm b" x="16" y="184">in Renyi <tspan class="mut" font-weight="400">· compiler/, run by the VM</tspan></text>
<rect class="box" x="16.5" y="192.5" width="89" height="28" rx="6"/><text class="sm" x="61" y="211" text-anchor="middle">lexer</text>
<rect class="box" x="122.5" y="192.5" width="89" height="28" rx="6"/><text class="sm" x="167" y="211" text-anchor="middle">parser</text>
<rect class="box" x="228.5" y="192.5" width="89" height="28" rx="6"/><text class="sm" x="273" y="211" text-anchor="middle">checker</text>
<rect class="box" x="334.5" y="192.5" width="89" height="28" rx="6"/><text class="sm" x="379" y="211" text-anchor="middle">emitter</text>
<path class="thin" d="M107 122.5 H116 M213 122.5 H222 M319 122.5 H328 M107 206.5 H116 M213 206.5 H222 M319 206.5 H328"/>
<path class="tip2" d="M116 119 L121 122.5 L116 126 Z M222 119 L227 122.5 L222 126 Z M328 119 L333 122.5 L328 126 Z M116 203 L121 206.5 L116 210 Z M222 203 L227 206.5 L222 210 Z M328 203 L333 206.5 L328 210 Z"/></g>
<g class="p2"><path class="wire" d="M220 237 V254"/><path class="tip" d="M216 254 L220 260 L224 254 Z"/>
<rect class="box" x="140.5" y="260.5" width="159" height="32" rx="16"/><text class="sm" x="220" y="281" text-anchor="middle">bytecode <tspan class="mono">.ryc</tspan></text>
<path class="wire" d="M220 293 V310"/><path class="tip" d="M216 310 L220 316 L224 310 Z"/></g>
<g class="p3"><rect class="frame" x="0.5" y="316.5" width="439" height="96" rx="12"/><text class="cap" x="16" y="338">THE VM</text>
<rect class="box" x="16.5" y="350.5" width="127" height="48" rx="8"/><text class="sm b" x="80" y="370" text-anchor="middle">interpreter</text><text class="mono sm mut" x="80" y="388" text-anchor="middle">--interpret</text>
<rect class="box" x="156.5" y="350.5" width="127" height="48" rx="8"/><text class="sm b" x="220" y="370" text-anchor="middle">template tier</text><text class="sm mut" x="220" y="388" text-anchor="middle">from the first call</text>
<rect class="tint2" x="296.5" y="350.5" width="127" height="48" rx="8"/><text class="sm b" x="360" y="370" text-anchor="middle">Cranelift tier</text><text class="sm mut" x="360" y="388" text-anchor="middle">when the code is hot</text>
<path class="thin" d="M285 374.5 H290"/><path class="tip2" d="M290 371 L295 374.5 L290 378 Z"/>
<rect class="dash" x="0.5" y="430.5" width="439" height="36" rx="10"/><text class="sm" x="16" y="453"><tspan class="mono b acc">renyi build</tspan> writes an image, <tspan class="mono">.ryi</tspan>: every function as machine code</text>
<path class="thin" d="M360 430 V419"/><path class="tip2" d="M356 419 L360 413 L364 419 Z"/></g>
</svg>
</figure>
</section>
