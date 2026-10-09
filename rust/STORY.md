# The story in play (B07, B08, B09, B10, B23)

> "Three kids, three warrior souls, one descent into the underworld to take their friend back from
> Krampus."

The review found the story bookended: strong in Scene I and the ending's poem, absent in between.
This is what carries it through play now (branch `story`, 2026-10-09). Code: `logic/tale.rs` (the
boys, their lines, Toby's things), `logic/after.rs` (the scenes at Hızır's), `audio/speech.rs` (the
voices), `game/toby.rs`, `game/counsel.rs` (the panel), `render/view/aftermath.rs`,
`render/view/toby.rs`, `render/hud/toby.rs`.

## 1. The cast and the names

| Who | Name | In the game |
|---|---|---|
| The friend Krampus takes | **Toby** (Tobias) | Kid 0 of the cast: red bobble cap, red scarf, blue coat. After the Book of Tobit, where the archangel Raphael walks beside young Tobias and Asmodeus is driven off; felt in the Lust scene and his bestiary page, never preached. |
| The old gatekeeper | **Hızır** (Khidr) | The ageless guide who comes to travellers in need when called. Always good, never lies; a kind, ancient, guilty gatekeeper, not a religious figure on screen. He names himself in Scene I's promise. |
| His dog | **Quitmer** | |
| The three boys | *not named yet* (open: D10) | Each goes by the warrior he carries: **MEHMED'S BOY** (kid 1: flat cap, yellow scarf, olive coat; steady), **RAGNAR'S BOY** (kid 2: fair, bare-headed, brick-red coat; hot-headed, he screamed "He took him!"), **MIYAMOTO'S BOY** (kid 3: ginger, teal cap and mittens, plum coat; quiet, imaginative). In Scene I the souls rise Mehmed, Ragnar, Miyamoto before the boys left to right and go into them (`tale::boy_of`). |
| The souls | jade (Mehmed), steel blue (Ragnar), rose (Miyamoto) | `tale::soul_color`: the boy's name and words in the panel, his outline in the glow. |
| The House | **The House of Seven Seals** | (was the House of the Devil.) Each prince's key holds one of its seven seals. Its door's banner: THE HOUSE IS EMPTY. / NO THRONE, NO DEVIL. ONLY THE WAY DOWN. |

## 2. One cosmology (B23)

> **"Every fear and every wrong in the world, from every land and every tongue, sinks down here
> and grows teeth."** (`story::COSMOLOGY`)

Why draugr, oni, cin, Tepegöz, the princes of the sins and Krampus share one underworld: nobody
rules them and nobody made them but us. Said in three places: Hızır at his door in Scene I ("Beyond
this door lies the world of the dead. Every fear..."), the guide's page of the Portals of Hell, and
under the bestiary's title. The poem pays it off: "Every monster in these depths was made of us". It
fits "there was never a devil": no lord over them is named anywhere (a test greps the sources).

## 3. Hızır (B09)

- Scene I's promise: "So it begins... This is my failure, and I feel the guilt of it. I will ask the
  archangels to send you help. And I am Hızır: call me whenever you need - I will come."
- HIZIR on the dialogue boxes, his panel and the wares; HIZIR SPEAKS heads his guide; CALL HIZIR in the
  pause; "Call Hızır" in the controls and tips; HIZIR'S WORKSHOP on the map; Hızır's wares on
  Continue; Hızır in the credits ("who kept the door, and kept his word"). Prose may still say "the
  old man" where it reads naturally.

## 4. The boys present in play (B07)

### 4.1 The silhouette in the glow

The boy's outline in spirit light, in his soul's colour (`WorldView::emit_boy_ghost`, the GHOST rig
look): on the **character screen** at each warrior's knee, inside a soft aura (brighter on the one
picked); at a **shrine** as the archangel's blessing comes (2.4 s, a little toward the camera so the
warrior does not hide him); and when the warrior **falls**, the boy stands where he fell until the
round begins again (3.2 s, larger than life so it reads from the top camera).

### 4.2 A line per place

Coming into a place, the boy whose warrior is played says one line in the voices' panel (his face,
his soul's colour, his voice; non-blocking, Enter skips; before the old man's next). Once each time
he comes in (`Game::boy_arrives`). The old man's counsel follows as before.

| Place | Mehmed's boy | Ragnar's boy | Miyamoto's boy |
|---|---|---|---|
| The Garden | His bells. I can hear Krampus's bells. We're coming, Toby. | He dragged Toby through here. We're taking him back. | It's like the park at night. Only the statues watch you. |
| The Portals of Hell | Seven doors. Seven keys. One Toby. Pick a door. | Lava everywhere. Toby would call this the best bad day ever. | Toby, I hope you're not as scared as I am. |
| Pride (Hall of Mirrors) | A hundred of me in the glass, and not one of them is Toby. Keep walking. | I always had to win. Toby always let me. I never said thanks. | The mirrors make me look big and brave. I'm not. I'm going anyway. |
| Envy (Serpent Grotto) | I used to wish I had Toby's bike. Now I just wish I had Toby. | Everyone liked Toby best. I minded that once. Not any more. | Toby shared everything. Even his last sweet. Even with me. |
| Wrath (Burning Field) | Stay cool. Angry kids make mistakes. Toby told me that. | I'm so angry I could burn too. No. Toby needs me thinking. | It's so hot. Toby hates the heat. He'd be moaning by now. |
| Sloth (Grey Mire) | Everything here wants me to lie down. I won't. Not till he's home. | My legs are so heavy. Toby would race me through this. Come on, legs. | So quiet. So grey. If I stop, I'll forget why I came. Toby. Toby. Toby. |
| Greed (Vaults of Mammon) | All this gold. I'd swap every coin of it for one more game with Toby. | Don't touch the gold. We came for Toby, not for this. | Toby would fill his pockets and give it all away by tea time. |
| Gluttony (Rotting Feast) | I'm starving. Toby always saved me half his sandwich. | Don't eat anything. Don't even look at it. | It smells like Mum's kitchen, and then it doesn't. Stay close, Toby. |
| Lust (Garden of Thorns) | Pretty roses, sharp thorns. Toby would say don't touch. So I won't. | Something keeps calling me to turn round. I won't look back. | It smells sweet. It wants me to forget. I won't forget you, Toby. |
| The Pit | Down and down. He's at the bottom. So that's where we go. | No devil. Just a hole. And Toby at the bottom of it. | Each floor is colder. Hold on, Toby. Nearly there. |
| Krampus's lair | TOBY! We're here! We came for you! | TOBY! Hold on! We're getting you out! | TOBY! It's us! We're here! |

In the lair Toby answers from the cage: **"I knew you'd come. I knew it!"**

### 4.3 A scene after each prince (`logic::after`)

After the prince falls and the hero walks out through the court's gate, the scene plays at Hızır's
workshop (the hero's own dressing of it), then the wares open as before. Esc skips it. 20 to 40 s:
the cast by their clips (`emit_old_man_at`, `emit_kid`, `emit_dog`), the three boys round the anvil
with Toby's thing on it, each with his warrior's soul standing behind him (softer than in Scene I),
Hızır across the anvil, Quitmer by the hearth. Every scene has one shape: GATHER (from behind the
anvil, the boys' faces and the souls), Hızır on the prince, SHOW (the finder on the thing, the camera
close on it), the other two boys, the last word, LEAVE (the camera draws back). The boys speak by
their part: the hero's own boy found it; the other two follow. On the LEVELS map: AT HIZIR'S - AFTER
<PRINCE>, seven stops. If the hero walked past the token, Quitmer fetched it: it is his by the scene.

**After Lucifer** (Pride; the drawing)
- HIZIR (warmly): Lucifer is down. He wanted every eye on him, and in the end no one was looking.
- THE FINDER (unfolding it): Toby left this between the mirrors. It's us. All four of us, holding hands.
- SECOND (grinning): He drew me taller than I am. He always does that.
- THIRD (softly): And he drew himself the smallest. Typical Toby.
- THE FINDER (firmly): When we find him, he can draw a new one. All four of us, going home.

**After Leviathan** (Envy; the tin soldier)
- HIZIR (gently): Leviathan wanted whatever anyone else had, and it ate him up from inside.
- THE FINDER: Look. Toby's tin soldier. The one with the chipped hat.
- SECOND (quietly): We all wanted that soldier. He let us take turns.
- THIRD: I chipped the hat. He said it made him look braver.
- HIZIR (warmly): A friend who shares his best thing... that is a friend worth going all this way for.

**After Amon** (Wrath; the mitten)
- HIZIR (gravely): Amon lived on anger. You would not give him any, so he starved.
- THE FINDER: His mitten. Red, like his hat. The fire got the end of it.
- SECOND (looking away): I was so angry in there. I wanted to smash everything.
- THIRD: Toby says you can be angry and still be kind. He's annoying like that.
- SECOND (smiling): ...He's right, though.

**After Belphegor** (Sloth; the chalk)
- HIZIR (gently): The grey wanted you to lie down and never get up. You did not.
- THE FINDER: Toby's chalk. He wrote on a stone with it: TOBY WAS HERE. And an arrow, pointing down.
- SECOND: He was still awake down there. Still thinking about us finding him.
- THIRD (firmly): Then we don't stop either. Not to sleep. Not for anything.
- HIZIR (kindly): Sit by my fire for a moment, then. Only a moment.

**After Mammon** (Greed; the football)
- HIZIR: Mammon is beaten, and all his gold could not buy him one more minute. What did you bring back?
- THE FINDER: Not gold. This. Toby's ball. It was lying in the coins like it was nothing.
- SECOND: That's the ball from the graveyard. He held on to it the whole way down.
- THIRD (grinning): He kicks it at my head every single day. I want him to do it again.
- HIZIR (warmly): Then keep it safe. A boy who holds on to his ball has not given up.

**After Beelzebub** (Gluttony; the bell)
- HIZIR: Beelzebub ate and ate, and he was never full. Now he is nothing at all.
- THE FINDER: This was in the feast. One of Krampus's bells, torn right off his sack.
- SECOND: Toby did that. He tore it off so it would fall where we'd find it.
- THIRD (softly): He's leaving us a trail. Even in a sack. Even scared.
- HIZIR (warmly): Your friend is braver than he knows. So are the three of you.

**After Asmodeus** (Lust; the scarf)
- HIZIR (gently): Long ago a boy called Tobias drove Asmodeus off with smoke, and an angel walked beside him all the way.
- THE FINDER: His scarf. The end of it, caught on the thorns. It still smells like his house.
- SECOND (quietly): Everything in there wanted me to turn round and stay. I nearly did.
- THIRD: Then I thought of Toby's silly laugh. And I kept going.
- HIZIR (smiling): That Tobias had one angel. Your Toby has three friends. I like his chances.

## 5. Toby's trail (B08)

One of his things in every realm, a few steps along the ground from where the hero comes in (7 to 16
steps; on the Burning Field only on the isles that never drift), on open floor, glowing in Toby's red
with motes rising. Walking over it takes it (a flag in the run's Warband save, `toby:`; no stat, no
path, no foe knows of it; Continue keeps it, a new adventure begins without). The boy knows it at
once (the line below, in the panel). The pause shows the seven in a row (TOBY'S TRAIL n/7), the
bestiary has a TOBY tab (who he is, his things found, Hızır's word), each prince's scene speaks of
its token, Toby names what the hero carries as his cage opens, and his football lies at their feet in
the end.

| Realm | Token | What the boy says as he picks it up |
|---|---|---|
| Pride | His drawing | The four of us, holding hands. He drew me taller than I am, and himself the smallest. |
| Envy | His tin soldier | The chipped hat. We all wanted it, and he let us take turns. |
| Wrath | His mitten | Red, like his hat. The fire got the end of it. |
| Sloth | His chalk (TOBY WAS HERE chalked beside it) | He wrote on a stone with it: TOBY WAS HERE, and an arrow pointing down. |
| Greed | His football | The ball from the graveyard. He held on to it all the way down. |
| Gluttony | A bell off Krampus's sack | He tore it off Krampus's sack, so it would fall where we would find it. |
| Lust | His scarf | The end of it, caught on the thorns. It still smells like his house. |

**The reunion.** As the cage door opens Toby says (`tale::toby_recognises`): all seven, "You found
everything! You followed my trail all the way down!"; some, "You came! You found my ball, my soldier
and my scarf - you followed my trail!" (the ball first); one, "You came! And you found my ball!";
none, "You came. You really came!".

## 6. The voices (B10, `audio::speech`)

No recorded voices. A line typing out is spoken syllable by syllable (a vowel run after a
consonant), each one a buzz through its vowel's two formants, its consonant (a hiss, a stop, a hum)
before it, a breath, an octave beneath or a growl where the voice has one.

- **Timbres** (`Talker::timbre`): Mehmed's boy 292 Hz steady; Ragnar's boy 258 Hz punchy and loudest;
  Miyamoto's boy 338 Hz soft and breathy; Toby 372 Hz brightest; the boys as one in Scene I; Hızır
  112 Hz, slow, reedy, an old man's tremor and a low octave; the princes: Lucifer bright and square,
  Leviathan hissing, Amon rasping, Belphegor slow, Mammon nasal, Beelzebub wet and deep, Asmodeus
  whispering; Krampus 64 Hz with a growl; the narrator plain; Quitmer keeps his snarl.
- **Moods** (`Mood`, from the beat's stage direction, `Mood::of`): fear (higher, wider, faster,
  trembling), anger (louder), resolve (steady), warmth (softer, slower), gravity, grief (quiet,
  breathy, slow), menace (low, cold; the princes' whispers), joy (high, bouncing; Toby).
- **Prosody**: the sentence falls as it goes; a question rises at its end; a full stop falls and
  lengthens; an exclamation peaks and is louder; capitals are stressed; the pitch wanders a little,
  the same every time.
- **The key beats**: "NO! He took him! LET HIM GO!" in terror (Ragnar's boy's punch, every word in
  capitals stressed); Hızır's promise warm through its weight; the reunion in Toby's joy.
- The voices' volume slider and the subtitles are as they were.

## 7. What still reads wrong

- **The boys have no names.** The panel says RAGNAR'S BOY and so on. Names would carry them better
  (D10 in BACKLOG.md).
- The scenes at Hızır's are of one shape and staged still: the boys stand and talk (Mixamo's talking
  clips), nobody picks the thing up or hands it round; the souls stand by but never move.
- The token in a realm lies near the way in, not on the way to the court; a player rarely misses it,
  but the realms' own trails (the Mire turning, the slabs drifting) may carry the ground about it.
- The speech is babble in a voice, not words; a fast line is a dense patter.
- On the character screen the boys stand in front of the warriors' knees; at a shrine the boy shows
  for a breath over the hero.
