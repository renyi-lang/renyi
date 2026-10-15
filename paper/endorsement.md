# The arXiv endorsement request

arXiv asks a first-time submitter to cs.PL for an endorsement from
someone who has published in that category in the last five years (and
more than three months ago). The submission form gives an endorsement
code and a link once the submission is started; the author sends the
link to the endorser, who follows it and confirms. The text below is the
email, to be sent from the author's own mailbox with the link filled in.

---

Subject: An endorsement request for an arXiv submission in cs.PL

Dear [name],

I am submitting an article to arXiv under cs.PL (cross-listed to cs.AI)
and need an endorsement, since this is my first submission in the
category. Would you be willing to endorse it? The link is

    [the endorsement link from arXiv]

The article is "Renyi: A Scripting Language for AI Agents with Declared,
Enforced and Recorded Effects". It presents a statically typed scripting
language whose function signatures declare effects as scoped, budgeted
capabilities: the checker refuses a function that uses what it does not
declare, the grant of `main` is the whole program's authority, and every
effect passes one boundary in the virtual machine at run time, so that a
run can be recorded, replayed offline and reproduced on another machine.
The implementation (a Rust toolchain of about a hundred thousand lines
with a bytecode VM, two native tiers, an ahead-of-time image format and a
WebAssembly build, and a compiler written again in the language and held
byte-equal to the Rust one by tests) is open source at
https://github.com/renyi-lang/renyi, released as version 0.4.0, and the
article reports its measures on that release: the reproduction of every
example workflow across operating systems, the first-run rate of agents
writing programs from the language's one-page skill file, and a
readability study.

The PDF is attached. The endorsement only says the article is
appropriate for the category; it is not a review, and I do not expect
one.

Thank you for considering it.

Zhe Zhang
zhezhang.dev@gmail.com
https://renyi-lang.org
