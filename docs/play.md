<!--
The play page (decision AW33, stage 33d): the toolchain's module for
WebAssembly runs in the browser. tools/site.py gives the page the layout
of `main.feature`, puts docs/assets/play.js, the game's sources and the
module built with `--module` under assets/ beside it, and the script
fills the canvas and the output; without it the page says what it
needs. The keys are the game's README's.
-->

<h1>Play</h1>

<section class="arcade">
<div class="prose">
<span class="num">01</span>
<h2 id="maze">The maze</h2>
<p>The showcase game, a raycasting maze of eleven modules written in Renyi, compiled in your browser by the same checker and emitter as the binary’s and run by the VM built for WebAssembly. Every frame is a pure function of the state and the input: the program hands it to the page through <code>screen.present</code> and suspends, the page draws the shapes and resumes the program with the keys held and the time elapsed.</p>
<p>Click the screen, then <kbd>W</kbd> and <kbd>S</kbd> or the arrows move, <kbd>A</kbd> and <kbd>D</kbd> turn, <kbd>Space</kbd> fires and <kbd>Escape</kbd> quits. Open a door, shoot what moves, find the exit.</p>
</div>
<div class="screen">
<canvas id="game" width="640" height="400" tabindex="0" aria-label="The game’s screen, 320 by 200 logical pixels">The game needs a browser with a canvas and WebAssembly.</canvas>
<div class="row"><button class="button primary" id="game-start" type="button" disabled>Loading the toolchain…</button><p class="status" id="game-status" aria-live="polite"></p></div>
<p class="status" id="game-rate"></p>
<noscript><p class="status">The game and the playground need JavaScript and WebAssembly.</p></noscript>
</div>
</section>

<section class="bench">
<div class="prose">
<span class="num">02</span>
<h2 id="playground">The playground</h2>
<p>Write a program, check it, run it: the diagnostics come with their fixes, as <code>renyi check</code> prints them. The page serves the console, the clock, random numbers, the environment and the screen; a <code>main</code> that needs anything else is refused before it runs, with the capability named. A program that presents frames plays on the screen above.</p>
</div>
<div class="desk">
<label class="sr" for="editor">The program</label>
<textarea id="editor" spellcheck="false" autocomplete="off" autocapitalize="off" rows="18">module playground
  purpose: Print the long words of a sentence, longest first.

import std.console

public function main() needs console
  purpose: Print every word of at least five letters, longest first.

  let words be "the agent writes it and you review it at a glance".split(" ")
  let long be
    for each word in words
    where word.length() is at least 5
    sorted by word.length() descending
    collect word
  console.print(long.join(", "))
end
</textarea>
<div class="row"><button class="button primary" id="run" type="button" disabled>Run</button><button class="button" id="check" type="button" disabled>Check</button><label class="args">Arguments <input id="arguments" type="text" autocomplete="off" spellcheck="false" placeholder="one two"></label><p class="status" id="run-status" aria-live="polite"></p></div>
<label class="sr" for="output">The output</label>
<pre class="output" id="output" tabindex="0" aria-live="polite"></pre>
</div>
</section>

<script src="assets/play.js" defer></script>
