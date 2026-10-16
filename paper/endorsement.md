# The arXiv endorsement request

arXiv asks a first-time submitter to cs.PL for an endorsement from
someone who has published in that category in the last five years (and
more than three months ago); endorsement is per category, so the
author's astro-ph record does not carry, and the author's institutional
email did not endorse automatically (2026-10-16). The submission form
gives an endorsement code and a link once the submission is started;
the author sends the link to the endorser, who follows it and confirms.
The text below is the email, to be sent from the author's own mailbox
with the link filled in, one endorser at a time. The first recipient is
Steve Zdancewic (University of Pennsylvania, an author of Quasar, which
the article cites as the closest work; his page spells the address
stevez at cis.upenn.edu); if he declines or does not answer within a
week, the same email goes to Osbert Bastani (the same paper), then to
Daan Leijen (Koka).

---

Subject: An endorsement request for an arXiv submission in cs.PL

Dear Professor Zdancewic,

I am submitting an article to arXiv under cs.PL (cross-listed to cs.AI)
and need an endorsement, since this is my first submission in the
category. Would you be willing to endorse it? The link is

    [the endorsement link from arXiv]

The article is "Renyi: A Scripting Language for AI Agents with Declared,
Enforced and Recorded Effects", by Brandon Mocco and me. It starts from
the premise of your Quasar paper, that an agent's program should
separate its logic from its effects, and takes it in another direction:
every function signature declares its effects as capabilities with a
scope, a budget and a data-flow guard; a checker refuses a function that
uses what it does not declare, across the program and its packages; and
at run time every effect passes one boundary in the virtual machine, so
that a run can be recorded, replayed offline and reproduced on another
machine from its manifest. The article cites Quasar, Pel and composable
effect handling as the closest work and states the differences. The
implementation (a Rust toolchain of about a hundred thousand lines with
a bytecode VM, two native tiers, an ahead-of-time image and a
WebAssembly build, and a compiler written again in the language and
held byte-equal to the Rust one by tests) is open source at
https://github.com/renyi-lang/renyi, released as version 0.4.0, and the
article reports its measures on that release.

The PDF is attached. The endorsement only says the article is
appropriate for the category; it is not a review, and I do not expect
one.

Thank you for considering it.

Zhe Zhang
zhezhang.dev@gmail.com
https://renyi-lang.org
