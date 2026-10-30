# What's new: the rounds

The build-and-critic rounds that made the mode (painting-company row 187). Each: what was built or
changed, how it was looked at (CI's simulator shots, the preview), what a fresh hard-to-impress critic
given only the pictures said would make the owner skip it (ranked by how much he'd notice), and what
the next round fixed. Stop: the critic's worst complaint is a quibble, or is overruled here with a
reason.

## Round 1 (October 30, 2026, ET)

**Built.** The module (pieces, ledger, session, store and courier, recorder and snapshot, views), the
Terra host, the queue seeded with twelve pieces, `WhatsNewTests`, `tools/whatsnew/whatsnew.py`, the
docs. Compiled with `[build-only]`; the first round of pictures comes from `[test WhatsNewTests]` on
the iPad and a hand-started run on both devices.

**Looked at.** `[test WhatsNewTests]` on the 13-inch iPad (run 212/213): the pill, the first piece, the
second, a picture taken, the mic tapped. The round test stopped at piece 2: Skip did not move on.

**Found before any critic.** The queue showed *1 of 5*: the seven packs' pieces were missing, and
with them the packs. A pack's `whatsNew` has no `id` of its own, the manifest's decoder required one,
and `PackStore` drops a pack whose manifest won't decode: a half-written piece could take a whole
pack off the workbench. The piece now decodes with empty defaults and `isSound` decides. The bar
came out as a dark grey box over the clouds (glass takes the scheme and what is behind it), four
lines of text, the gold question unreadable on it; the pill and the taps came out light. Skip and
Didn't work were grey words on the sky, 16 points tall, at the screen's bottom edge. A picture of the
screen had the globe in it (the Metal layers made readable for the frame: the one thing that could
not be reasoned out off the Mac).

**Changed.** Everything of the mode sits on the caption's paper (a little translucent, light in any
scheme), with the atlas's gold as an ink dark enough to read; `what` is two lines at most; Skip and
Didn't work are paper buttons, 36 points tall; the test asserts the count after each Skip and
answers the microphone sheet from an inert touch (a touch on the globe had lit Turkey).

## Round 2

**Looked at.** `[test WhatsNewTests]` on the iPad again (run 214): both tests pass; twelve pieces;
the pill, the first three pieces, the picture, the end card, the list, one piece played alone.

**Found.** The paper chrome reads over clouds, sea and land, and the gold question with it; Skip and
Didn't work are buttons now. The picture's thumbnail shows the globe. But seven of the twelve pieces
showed the plain globe and its own caption: a pack's entry has no `start`, and only the tool
(`whatsnew.py`) had the default of running the pack; the host ran nothing. The `what` lines, written
for a reader with time, were cut with an ellipsis at two lines even on the iPad.

**Changed.** The host gives a pack's piece the pack to run unless its `start` says otherwise. Every
`what` is shortened to two lines of the iPad's bar. The third round runs on both devices (a
hand-started run, `device: both`), and gets the first critic.

## Round 3

**Looked at.** A hand-started run on both devices (run 216): every test passes on the 13-inch iPad and
the iPhone 17 Pro; eighteen pictures.

**Found.** The pack pieces start their packs (the herds open on the first camelid, 41 million years
ago; the vine on its date). On the phone the bar's `what` is cut mid-sentence at two lines and the
question takes two more: four lines over the globe. The list's dates read as ISO (`2026-10-29`) and
its first row says *Nothing new* with a line meant for *Play all*. Wikipedia's words on Georgia gave
the page that lists every Georgia: three countries' articles were disambiguation pages (fixed in
`tools/wikipedia/articles.txt`, the index rebuilt). The first critic reads these pictures.

**Changed.** The phone's bar shows three whole lines of `what` rather than two cut; the list says
*Landed Oct 29 · nice Oct 29* and, with nothing due, *Nothing new in this build*.

## Round 4

**The first critic** (round 3's eighteen pictures, both devices), ranked: (1) the pieces arrive at
zero, before they have anything to show (the vine a seed, the herd specks, the lit country under
doorways and cloud), so every screen invites Skip; (2) the second piece shows a defect of its own
(Georgia's disambiguation page); (3) the band is a toolbar of seven equal tiles, two faces alike,
the globe's names leaking between them; (4) the bar is three lines of small text and the one line
that matters, the question, is the least legible, and a yes/no question the taps can't answer; (5)
the card's primary is configuration and its tally a reproach; (6) twelve pieces with no sense of
time; (7) everything else on the globe clutters the piece; (8) the close, Open and Say unexplained;
(9) the list written by an engineer (ISO dates, file counts, a riddle under "Nothing new"); (10) the
phone's chrome eats the piece; (11) the microphone prompt's paragraph; (12) the pill jammed against
the buttons. Good: the pill, Picture becoming Taken, the card's headline. Verdict: a real gap.

**Changed, in that order.** Starts at their moments: the vine with 3,500 years grown, the country
centred at zoom 5 with the clouds off, the other doorways taken off the globe and the clouds thinned
while any piece plays (`doorways: true` keeps them: the living globe). Georgia's article (and
Saint-Martin's, Vatican's) fixed in Wikipedia's words. The band: three big verdicts set as words on
three tints (warm, green, grey), Say and Picture small at the ends, Skip and Didn't work plain
words, the whole on a near-opaque strip of paper the globe keeps its names clear of. The bar: one
title line and the question as its main line in roman gold; `what` only when there is no question;
the questions rewritten as things to judge; the close is *Later*, Open is *Panel*. The card: *Done*
first, *Keep going (7)* while pieces wait, the tally in words without the skips, sending a quiet
line. A round is five pieces; the pill counts all that wait. The list: one line a piece, *Oct 29*, a
glyph or a dot. The picture stays in the band's camera button. The microphone's reason is one
sentence. Not taken: the microphone prompt itself (the system's); the question's yes/no form is met
by rewriting the questions rather than the taps.

**The unbriefed QA pass** (qa-explore, release build, iPad, 8 minutes, told only that the build had
something new to show; run 40987479322). It found the pill and the round on its own ("a guided
tour"), read the bar and the band as meant, and reported two things: the first tap on the pill did
nothing and the second opened the round (seen once, not reproduced here: the UI tests' first tap
opens it every time; left as a watch item), and touching the camel's doorway from the living-globe
piece opened the camel's lens over the bar and the band, each half hiding the other. The lens is an
excursion: the bar and the band now step aside while one is open and come back when it closes.

**Round 4 looked at** (`[test WhatsNewTests]`, the iPad, run 219): both tests pass. The vine arrives
with its branches across Anatolia, the Levant and the Gulf at 2,358 BCE; Georgia is centred, lit,
clear of cloud, with its own first sentence; the band is three words on three tints and the globe's
names stay out of it; the card leads with Done and offers Keep going (7); the list is a line a piece.
The second critic reads these; a hand-started run photographs the phone too.

## Round 5

**The second critic** (round 4's nine iPad pictures), ranked: (1) the bar's "Judge…" lines are exam
questions over an applause meter, two lines long, two things at once: homework; (2) the card reports
"1 nice" for a round of five, offers Done as a tap that only dismisses, and admits the answers went
nowhere; (3) Later, Skip and Panel are three unexplained words, and a piece alone shows "1 of 1" with
a dead chevron; (4) Skip and Didn't work are small grey text with no shape; (5) some pieces open
where the asked judgment can't be made (the vine asked far and close, the herds "come close" when
the animal fills the screen); (6) piece one is the globe he already knows, its old caption under the
new bar; (7) Say gives no recording state; (8) the foot's stack eats the bottom third; (9) the list is
a settings form (a date column of identical dates, an unexplained dot, Sending second, Seen off the
bottom); (10) the pill's "Show me" too faint; (11) the herds' ground blurred at the pack's zoom; (12)
a second tap on the picture unexplained. Good: the three coloured pills, the finite round, the
cleared globe. Verdict: a real gap.

**Changed.** The line is a hint of where to look, five or six words, one line, never a question
("One animal, close; the date slows"); the taps are the question, and the contract says so (the
validator holds `ask` to 60 characters). The card shows the answers as words in order ("Nice · Skip
· Skip · Skip · Skip"), one gold button sends them (with a token) or shares them (without), Keep
going and Close beside; the "set up sending" line is gone (the workbench has it). One exit, ✕, with
Skip for a piece; Panel is About; a piece alone says "Again" and has no chevron. Skip and Didn't
work are small outlined pills. Say reads "Stop 0:04" in red while recording. The list leads with
Seen (what was said, when), then New, Sending last, no date column. The living globe is last among
the seeds (it landed first). The pill is one phrase in one weight.
**Overruled, with reasons.** The pack's caption at the foot stays as it is while a piece plays: it is
the piece's own words, and a pack's date strip is its instrument. The herds' zoom is the pack's
choice (its makers open close on one animal); the ground's blur there is the ground's, noted for
them. The microphone permission is the system's; it is asked on the first Say, not before the round
(a sheet before anything has been shown would be the homework).

**Round 5 looked at** (both devices, run 222): the pictures are as meant on both (the hint one line,
the band's words, the card "Nice · Skipped · Skipped · Skipped · Skipped / 7 more wait for next time /
Share these 5"). The round test failed at its last check on both devices: a stage alive after the
round. The last piece was a pack skipped within a second of starting; the task preparing its files
held the stage until the preparing was done, seconds after the reader had left. The task holds it
weakly now, so the Live counts are back the moment the round ends. Not a leak, but exactly what the
Live rule is for: a fast skipper would have hit it.

## Round 6

**The third critic** (round 5's fourteen pictures, both devices): the core loop works and nothing
structural is wrong; the worst that remains is one screen, the end card (verdicts without the
pieces' names; Share as the primary when the answers are saved anyway; "7 more wait for next time"
contradicting Keep going), and on the phone the globe shrunk between the bar and the band so the
piece is a thumbnail; then the band shifting under a thumb when Say or Picture change width, quiet
pieces with no instruction, Skip's consequence unsaid, 10-point labels, the picture's affordance,
Meh's grey. "Both are afternoon fixes, not a redesign."

**Changed.** The card names each piece beside what was said of it; the gold button is Send when a
token is set, else Close with the answers saved here (Share beside it); the foot says where the
skipped went and that more wait "when you like". On a phone the camera comes in by a third, the
share of the globe's height the chrome takes, so the piece is the size its makers framed on an iPad.
Say and Picture keep one width in every state; the picture's thumbnail says Retake; labels are 12
points; the quiet pieces' hints say what to do ("Tap near a place: its name comes for a while").
Meh's grey is a shade warmer.
