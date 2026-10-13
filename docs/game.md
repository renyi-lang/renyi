<!--
The game's page (decision AW35, stage 36d): the showcase game on a page
of its own, the screen as wide as the window allows, with sound.
tools/site.py gives the page the layout `game` (no group, no pager: the
page is its own), puts docs/assets/play.js, the game's sources, the art
and the module built with `--module` under assets/ beside it, and the
script fills the canvas and plays the sounds; without the module the
page says what it needs. The keys are the game's README's.
-->

<p class="eyebrow"><a href="play.md">Play</a> · The showcase game</p>
<h1>Raycaster</h1>

<div class="prose">
<p class="lead">A Doom-style maze written in Renyi, compiled in your browser by the same checker and emitter as the binary’s and run by the VM built for WebAssembly: textured walls, a demon that hunts you, a shotgun, the status bar and the sounds, with the art of <a href="https://github.com/freedoom/freedoom">Freedoom</a>. Every frame is a pure function of the state and the keys held: the program hands it to the page through <code>screen.present</code> and suspends, the page draws the shapes and resumes the program with the input.</p>
</div>

<div class="stage" id="stage">
<canvas id="game" width="640" height="400" tabindex="0" aria-label="The game’s screen, 320 by 200 logical pixels">The game needs a browser with a canvas and WebAssembly.</canvas>
</div>

<div class="row controls">
<button class="button primary" id="game-start" type="button" disabled>Loading the toolchain and the art…</button>
<button class="button" id="game-full" type="button">Full screen</button>
<label class="check"><input id="game-sound" type="checkbox" checked> Sound</label>
<p class="status" id="game-status" aria-live="polite"></p>
<p class="status" id="game-rate"></p>
</div>

<div class="prose keys">
<p><kbd>Space</kbd> or <kbd>F</kbd> fires, and starts the game from its title screen. <kbd>W</kbd> and <kbd>S</kbd> or the arrows move, <kbd>A</kbd> and <kbd>D</kbd> turn, <kbd>Escape</kbd> or <kbd>Q</kbd> quits. Open a door by walking into it, shoot what moves, find the exit.</p>
<p>The game is the twelve modules of <a href="https://github.com/renyi-lang/renyi/tree/main/showcase/raycaster"><code>showcase/raycaster</code></a>, about two thousand lines of Renyi, run by the interpreter of the toolchain’s module; the same program plays at the terminal under <code>renyi run</code>, where the frames are drawn as text. The images and the sounds are Freedoom’s, under its BSD license. Nothing leaves the page.</p>
</div>

<noscript><p class="status">The game needs JavaScript and WebAssembly.</p></noscript>

<script src="assets/play.js" defer></script>
