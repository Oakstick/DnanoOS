//! Scene I's camera: each beat a film shot (RENDERING.md, "Scene I's camera"). Where it
//! stands, what it looks at, what is in focus, and its lens (a field of view, a lean, a depth of
//! field) are worked out from the moment alone (`StoryView`), so a shot plays the same every
//! time, a capture of any instant shows what play shows, and the tests can walk every shot
//! through time. The cuts and the story's timing are the story's (`logic::story`); the camera
//! only chooses how each moment is seen.
//!
//! It frames the cast by their marks (`story::marks`) and the beats' timings (`stage`), so it
//! follows the ball, Krampus and the boys without reaching into how they are drawn.

use super::story::marks::*;
use super::story::{StoryView, GATEWAY, GRAVEYARD};
use super::*;
use crate::logic::story::{stage, Scene, Shot, Speaker};

/// The depth of field: how far behind the in-focus plane the blur begins and where it is full
/// (world units), the most it blurs (pixels at 1080p), and the same in front of it (`near_clear`
/// 0: nothing in front blurs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dof {
    pub clear: f32,
    pub full: f32,
    pub px: f32,
    pub near_clear: f32,
    pub near_full: f32,
}

impl Dof {
    /// Blurring behind the in-focus plane only.
    pub const fn behind(clear: f32, full: f32, px: f32) -> Self {
        Self { clear, full, px, near_clear: 0.0, near_full: 1.0 }
    }

    /// The same, and whatever stands this close in front of the camera's subject softening too (a
    /// shoulder in the foreground of a shot over it).
    const fn and_front(self, clear: f32, full: f32) -> Self {
        Self { near_clear: clear, near_full: full, ..self }
    }
}

/// A shot's lens: its vertical field of view (degrees), how far it leans (radians, about the
/// view), and its depth of field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Lens {
    pub fov: f32,
    pub roll: f32,
    pub dof: Dof,
}

/// The story's camera for one moment.
#[derive(Clone, Copy, Debug)]
pub(super) struct StoryCam {
    pub eye: Vec3,
    pub look: Vec3,
    pub focus: Vec3,
    pub lens: Lens,
}

/// The whole set sharp but the far distance (the wides, the crane, Scene II): as composed before.
const DEEP: Dof = Dof::behind(5.0, 20.0, 3.5);
/// A medium shot: the action sharp, the set behind it softening a few metres back.
const MEDIUM: Dof = Dof::behind(1.4, 6.0, 4.5);
/// Faces and close things: what is behind them a metre or two back goes soft.
const SHALLOW: Dof = Dof::behind(0.45, 2.4, 6.0);

/// Lenses, as vertical fields of view: wide for the crane and the reveals, long for faces.
const WIDE: f32 = 56.0;
const NORMAL: f32 = 46.0;
const LONG: f32 = 34.0;
const PORTRAIT: f32 = 27.0;

fn lerp_dof(a: Dof, b: Dof, u: f32) -> Dof {
    let l = |x: f32, y: f32| x + (y - x) * u;
    Dof {
        clear: l(a.clear, b.clear),
        full: l(a.full, b.full),
        px: l(a.px, b.px),
        near_clear: l(a.near_clear, b.near_clear),
        near_full: l(a.near_full, b.near_full),
    }
}

/// Smoothstep: a move easing in and out.
fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smootherstep: a slower start and settle, for the long camera moves.
fn ease(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn ease_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(2)
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// A camera placing within a shot (set coordinates): eye, look, field of view.
#[derive(Clone, Copy, Debug)]
struct Key {
    eye: Vec3,
    look: Vec3,
    fov: f32,
}

const fn key(eye: Vec3, look: Vec3, fov: f32) -> Key {
    Key { eye, look, fov }
}

impl Key {
    /// Part way from this placing to `b`.
    fn to(self, b: Key, u: f32) -> Key {
        Key { eye: self.eye.lerp(b.eye, u), look: self.look.lerp(b.look, u), fov: self.fov + (b.fov - self.fov) * u }
    }
}

/// A shot at one moment, in its set's coordinates: the placing, what is in focus, the depth of
/// field, how much the hand holding it moves (1 a steady hand), and a lean.
struct Take {
    key: Key,
    focus: Vec3,
    dof: Dof,
    hand: f32,
    roll: f32,
}

fn take(key: Key, focus: Vec3, dof: Dof) -> Take {
    Take { key, focus, dof, hand: 1.0, roll: 0.0 }
}

/// Held by hand: a slow drift that never quite settles (eye, look, lean), `amount` times a
/// steady hand's.
fn handheld(age: f32, amount: f32) -> (Vec3, Vec3, f32) {
    let eye = v(
        (age * 0.53).sin() * 0.018 + (age * 1.37 + 1.0).sin() * 0.005,
        (age * 0.71).sin() * 0.012 + (age * 1.93 + 2.0).sin() * 0.004,
        (age * 0.37).cos() * 0.01,
    );
    let look = v((age * 0.61 + 2.0).sin() * 0.012, (age * 0.83 + 0.5).sin() * 0.008, 0.0);
    (eye * amount, look * amount, (age * 0.47).sin() * 0.004 * amount)
}

impl WorldView {
    /// The story's camera for this moment: where it is, what it looks at, what is in focus, and
    /// its lens.
    pub(super) fn story_camera(&self, sv: StoryView) -> StoryCam {
        if sv.scene == Scene::Two {
            // Scene II keeps the camera it was composed with (ascent.rs), held by hand.
            let (eye, look, focus) = self.ascent_camera(sv);
            let sway = v((sv.age * 0.53).sin() * 0.018, (sv.age * 0.71).sin() * 0.012, (sv.age * 0.37).cos() * 0.01);
            return StoryCam { eye: eye + sway, look: look + sway * 0.5, focus, lens: Lens { fov: FOV_DEGREES, roll: 0.0, dof: DEEP } };
        }
        scene_one(sv)
    }
}

/// Scene I's camera at this moment.
pub(super) fn scene_one(sv: StoryView) -> StoryCam {
    let t = sv.t;
    let shot = sv.shot();
    let (origin, take) = match shot {
        Shot::Title => (GRAVEYARD, establish(t - Scene::One.beats()[0].hold as f32)),
        Shot::Establish => (GRAVEYARD, establish(t)),
        Shot::Taken => (GRAVEYARD, taken(t)),
        Shot::Scream => (GRAVEYARD, scream(t)),
        Shot::Pursuit => (GRAVEYARD, pursuit(t)),
        Shot::Arrive => (GATEWAY, arrive(t)),
        Shot::Snarl => (GATEWAY, snarl(t)),
        Shot::Easy => (GATEWAY, easy(t)),
        Shot::OldMan => (GATEWAY, old_man(t, said_before(sv, Speaker::OldMan))),
        Shot::Boys => (GATEWAY, boys(t, said_before(sv, Speaker::Boys))),
        Shot::Souls => (GATEWAY, souls(t)),
        Shot::Promise => (GATEWAY, promise(t)),
        Shot::Descent => (GATEWAY, descent(t)),
        _ => (GATEWAY, take(key(v(0.0, 1.3, 3.5), v(0.0, 1.0, -3.0), NORMAL), v(0.0, 1.0, 0.0), DEEP)),
    };
    let (sway, glance, lean) = handheld(sv.age, take.hand);
    StoryCam {
        eye: origin + take.key.eye + sway,
        look: origin + take.key.look + sway * 0.5 + glance,
        focus: origin + take.focus,
        lens: Lens { fov: take.key.fov, roll: take.roll + lean, dof: take.dof },
    }
}

/// How many lines `who` has spoken in the scene before this beat (which way a conversation's
/// cutting goes, and which boy speaks).
fn said_before(sv: StoryView, who: Speaker) -> usize {
    let beats = sv.scene.beats();
    beats[..sv.beat.min(beats.len())].iter().filter(|b| b.speaker == who).count()
}

// ------------------------------------------------------------ the graveyard

/// Where the ball is, `t` seconds into the establishing shot (as `emit_graveyard` plays it).
fn ball_at(t: f32) -> Vec3 {
    let feet = |i: usize| KID_HOME[i] + v(0.0, BALL_R, 0.0);
    let mut p = feet(0) + v(0.08, 0.0, -0.1);
    for (n, &(a, b)) in PASSES.iter().enumerate() {
        let k = stage::KICKS[n];
        if t >= k {
            let u = ((t - k) / 0.62).min(1.0);
            p = feet(a).lerp(feet(b), ease_out(u)) + v(0.0, (u * PI).sin() * 0.12, 0.0);
        }
    }
    if t >= stage::MISKICK {
        let u = ((t - stage::MISKICK) / (stage::BALL_AT_GATE - stage::MISKICK)).min(1.0);
        let gate = v(0.02, BALL_R, -2.0);
        p = feet(2).lerp(gate, u);
        if t >= stage::BALL_AT_GATE {
            p = gate.lerp(BALL_REST, ease_out((t - stage::BALL_AT_GATE) / 1.1));
        }
    }
    p
}

/// The boy who fetches the ball: at home, running to the gate, peering through it.
fn fetcher_at(t: f32) -> Vec3 {
    KID_HOME[0].lerp(GATE_SPOT, ((t - stage::FETCH) / (stage::AT_GATE - stage::FETCH)).clamp(0.0, 1.0))
}

/// Where Krampus stands, `k` seconds into the taking (as `emit_graveyard` plays it).
fn krampus_at(k: f32) -> Vec3 {
    if k < stage::RUSH {
        K_FOG
    } else if k < stage::GATE_BURST {
        K_FOG.lerp(K_GATE, ease_in((k - stage::RUSH) / (stage::GATE_BURST - stage::RUSH)))
    } else if k < stage::TURN {
        K_GATE.lerp(K_GRAB, ease_out((k - stage::GATE_BURST) / (stage::GRAB - stage::GATE_BURST)))
    } else {
        K_GRAB.lerp(K_GONE, ease_in((k - stage::TURN - 0.15) / (stage::GONE - stage::TURN - 0.15)))
    }
}

/// The crane's top (over the graves, on the dusk) and its foot (down among the friends, at a
/// child's eye height).
const CRANE_TOP: Key = key(Vec3::new(4.8, 6.4, 9.6), Vec3::new(0.0, 1.3, -6.0), WIDE);
const CRANE_FOOT: Key = key(Vec3::new(1.72, 0.66, 3.3), Vec3::new(-0.2, 0.46, 1.3), 42.0);

/// The establishing shot, `c` seconds into it (negative in the title card before it, where the
/// crane begins to move as the picture comes up).
fn establish(c: f32) -> Take {
    if c < stage::MISKICK + 0.35 {
        // Down from over the graves to the friends at their game, then a slow push in as the
        // passes go; the focus rides on the ball.
        let u = ease(-0.7, 3.3, c);
        let mut k = CRANE_TOP.to(CRANE_FOOT, u);
        // (The crane comes in over the bench by the lane before it comes down, settling level in
        // front of it.)
        let across = ease(-0.7, 2.7, c);
        k.eye.x = CRANE_TOP.eye.x + (CRANE_FOOT.eye.x - CRANE_TOP.eye.x) * across;
        k.eye.z = CRANE_TOP.eye.z + (CRANE_FOOT.eye.z - CRANE_TOP.eye.z) * across;
        let push = ease(1.6, stage::MISKICK + 0.5, c);
        k.eye += (k.look - k.eye).normalize() * 0.55 * push;
        k.fov -= 4.0 * push;
        let mut t = take(k, ball_at(c.max(0.0)), lerp_dof(DEEP, MEDIUM, u));
        t.hand = 0.6 + 0.4 * u;
        t
    } else if c < stage::BALL_AT_GATE {
        // The miskick: low behind the ball as it hops away to the gate.
        let b = ball_at(c);
        let eye = v(b.x * 0.6 + 0.22, 0.24, b.z + 1.05);
        take(key(eye, v(0.06, 0.42, -2.6), 40.0), b, SHALLOW)
    } else {
        // Through the bars, from among the graves: the ball rattles through toward us and rolls
        // to rest; the boy runs up to the gate and peers through, his friends soft behind him.
        let u = ease(stage::BALL_AT_GATE, 7.4, c);
        let k = key(v(-0.3, 0.34, -4.75), v(0.14, 0.6, -1.6), 32.0).to(key(v(-0.16, 0.42, -4.05), v(0.13, 0.62, -1.5), 29.0), u);
        let pull = smooth(5.55, 6.3, c);
        let focus = ball_at(c).lerp(fetcher_at(c) + v(0.0, 0.55, 0.0), pull);
        take(k, focus, SHALLOW)
    }
}

fn taken(t: f32) -> Take {
    let him = krampus_at(t);
    if t < stage::RUSH {
        // Low behind the three, their backs dark against the gate: the bells, then the eyes
        // opening in the fog beyond, and the focus going out to them.
        let u = ease(0.0, stage::RUSH, t);
        let k = key(v(-0.06, 0.42, 3.6), v(0.22, 1.02, -4.5), 30.0).to(key(v(-0.06, 0.43, 3.3), v(0.24, 1.06, -4.5), 29.0), u);
        let focus = v(0.12, 0.6, -1.42).lerp(K_FOG + v(0.0, 1.7, 0.0), smooth(stage::EYES, stage::EYES + 0.45, t));
        take(k, focus, Dof::behind(1.0, 4.0, 5.0).and_front(0.8, 2.0))
    } else if t < stage::GRAB - 0.02 {
        // The rush: low at the boy's back as the gate bursts open toward us.
        let u = ease(stage::RUSH, stage::GRAB, t);
        let k = key(v(0.66, 0.42, -0.4), v(0.08, 1.0, -2.6), NORMAL).to(key(v(0.64, 0.4, -0.5), v(0.1, 1.15, -2.4), NORMAL - 2.0), u);
        let mut tk = take(k, him + v(0.0, 1.2, 0.0), MEDIUM);
        // A jolt as the gate gives.
        tk.hand = 1.0 + 6.0 * (-(t - stage::GATE_BURST).max(0.0) * 9.0).exp() * smooth(stage::GATE_BURST - 0.02, stage::GATE_BURST, t);
        tk
    } else if t < stage::TURN + 0.25 {
        // Cut in: down under him, looking up, the boy held high in his claws.
        let u = ease(stage::GRAB, stage::TURN + 0.25, t);
        let k = key(v(0.7, 0.2, -0.66), v(0.1, 1.5, -2.1), 52.0).to(key(v(0.66, 0.18, -0.8), v(0.06, 1.62, -2.1), 50.0), u);
        let mut tk = take(k, K_GRAB + v(0.0, 1.3, 0.0), MEDIUM);
        tk.hand = 1.8;
        tk.roll = -0.05 * (1.0 - u);
        tk
    } else {
        // Away into the graves with him, down the path past the gate, until the fog has him.
        let u = ease(stage::TURN + 0.25, 4.6, t);
        let k = key(v(0.3, 0.95, -0.45), v(0.2, 1.0, -4.0), 40.0).to(key(v(0.3, 0.9, -1.05), v(0.6, 0.9, -7.5), 33.0), u);
        let k = Key { look: k.look.lerp(him + v(0.0, 0.95, 0.0), 0.6 * (1.0 - smooth(stage::GONE, 4.2, t))), ..k };
        take(k, him + v(0.0, 1.0, 0.0), MEDIUM)
    }
}

/// The three left behind: a push in on their faces, low, the gate's lantern on them, to a close
/// on the middle one as the line lands.
fn scream(t: f32) -> Take {
    let u = ease(0.0, 2.8, t);
    let k = key(v(-0.3, 0.42, -0.6), v(-0.3, 0.55, 2.4), 44.0).to(key(v(-0.46, 0.48, 1.5), v(-0.45, 0.53, 2.62), LONG), u);
    // (Then a breath of a push more, for as long as the line is up.)
    let k = Key { eye: k.eye + v(0.0, 0.0, 0.08) * ease(2.8, 9.0, t), ..k };
    let mut tk = take(k, v(-0.45, 0.6, 2.6), SHALLOW);
    tk.hand = 1.3;
    tk
}

/// Where the three chasing boys are, `p` seconds into the chase, on average (as `emit_graveyard`
/// runs them).
fn pack_at(p: f32) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for i in 1..4usize {
        let start = (i - 1) as f32 * 0.14;
        let u = ((p - start) / 2.4).clamp(0.0, 1.0);
        let from = KID_HOME[i] + v(0.0, 0.0, 0.45);
        let gate = v((i as f32 - 2.0) * 0.28, 0.0, -2.0);
        let beyond = v((i as f32 - 2.0) * 0.5, 0.0, -5.6);
        sum += if u < 0.5 { from.lerp(gate, u * 2.0) } else { gate.lerp(beyond, (u - 0.5) * 2.0) };
    }
    sum / 3.0
}

/// Running with them: low behind, hand-held and jolting, through the gates into the fog, and the
/// flash, the ground gone, the fall.
fn pursuit(t: f32) -> Take {
    let mid = pack_at(t - 0.22);
    let ahead = pack_at(t + 0.45);
    let stride = t * 8.6;
    // (Further back as they start away, closing on them as the camera gets going.)
    let behind = 1.75 - 0.6 * ease(0.0, 1.3, t);
    let mut eye = v(mid.x * 0.5 + 0.3, 0.52, mid.z + behind) + v(stride.sin() * 0.025, stride.cos().abs() * 0.035, 0.0);
    let mut look = v(ahead.x * 0.5, 0.5, ahead.z - 1.8);
    // The flash: the floor goes, the view tips down and over into the white.
    let fall = ease(stage::FLASH, stage::FLASH + 0.75, t);
    eye.y -= 0.28 * fall;
    look.y -= 1.6 * fall;
    let mut tk = take(key(eye, look, 50.0 + 8.0 * fall), mid + v(0.0, 0.5, 0.0), MEDIUM);
    tk.hand = 2.6;
    tk.roll = 0.55 * fall + (stride * 0.5).sin() * 0.012;
    tk
}

// ------------------------------------------------------------ the gateway

/// The old man's lantern, by his stool (`gateway_lights`).
const LANTERN: Vec3 = Vec3::new(2.4, 0.18, -2.25);
/// His face, where he stands.
fn old_face() -> Vec3 {
    OLD_MAN_AT + v(0.0, 1.1, 0.0)
}

fn arrive(t: f32) -> Take {
    if t < stage::REVEAL {
        // From the floor, looking up at the rent they are thrown out of, onto the flags around us.
        let u = ease(0.0, stage::REVEAL, t);
        let k = key(v(0.3, 0.16, -0.25), v(0.0, 1.0, 3.6), WIDE).to(key(v(0.34, 0.2, -0.45), v(0.0, 0.8, 3.0), WIDE - 3.0), u);
        let mut tk = take(k, v(0.05, 0.4, 1.7), MEDIUM);
        tk.hand = 1.4;
        tk
    } else {
        // The room, over their heads: the door under its skull, the braziers, the dog waking on his
        // straw and coming at them; the old man asleep in the gloom of his corner.
        let u = ease(stage::REVEAL, 5.6, t);
        let k = key(v(0.25, 1.02, 3.55), v(-0.15, 0.95, -2.5), WIDE).to(key(v(0.15, 0.96, 3.1), v(-0.3, 0.85, -2.5), WIDE - 3.0), u);
        let dog = DOG_BED.lerp(DOG_LUNGE, smooth(stage::GROWL - 0.4, stage::GROWL + 0.4, t)) + v(0.0, 0.55, 0.0);
        let focus = v(0.0, 1.0, -3.2).lerp(dog, smooth(stage::GROWL - 0.3, stage::GROWL + 0.4, t));
        take(k, focus, lerp_dof(DEEP, MEDIUM, smooth(stage::GROWL, stage::GROWL + 0.6, t)))
    }
}

/// The dog at the end of his chain, coming at us: low, near the floor, in his path.
fn snarl(t: f32) -> Take {
    let u = ease(0.0, 4.0, t);
    let toward = (BOYS_MID - DOG_LUNGE).normalize();
    let head = DOG_LUNGE + toward * 0.62 + v(0.0, 0.62, 0.0);
    let k = key(v(-0.58, 0.3, 0.1), v(-1.42, 0.57, -1.2), 46.0).to(key(v(-0.52, 0.32, 0.22), v(-1.41, 0.58, -1.2), 44.0), u);
    let mut tk = take(k, head, SHALLOW);
    tk.hand = 1.8;
    tk
}

/// "Easy, Quitmer": the old man found by his lantern first, low, then up to his face.
fn easy(t: f32) -> Take {
    let u = ease(0.4, 3.4, t);
    let k = key(v(0.72, 0.28, -1.2), LANTERN, 40.0).to(key(v(0.2, 1.0, -1.25), old_face() + v(-0.04, -0.17, 0.0), LONG), u);
    let focus = LANTERN.lerp(old_face(), smooth(1.0, 2.8, t));
    take(k, focus, SHALLOW.and_front(0.3, 0.9))
}

/// The old man speaking (his `n`th line): over the boys' shoulders, or a clean single, in turn.
fn old_man(t: f32, n: usize) -> Take {
    let face = old_face();
    let (k, dof) = if n % 2 == 1 {
        // Clean, at about his height: him, the gate's dark behind.
        (key(v(0.9, 1.0, -0.32), face + v(0.0, -0.13, 0.0), 30.0), SHALLOW)
    } else {
        // Over the boys' shoulders, low, a long lens across the room, the nearest boy soft.
        (key(v(0.86, 0.66, 2.3), face + v(-0.1, -0.16, 0.0), 26.0), SHALLOW.and_front(1.0, 2.5))
    };
    let push = ease(0.0, 7.0, t);
    let k = Key { eye: k.eye + (k.look - k.eye) * 0.07 * push, ..k };
    take(k, face, dof)
}

/// Where the boy who speaks the `n`th of the boys' lines stands, and his face.
fn speaker_face(n: usize) -> Vec3 {
    LAND[n % 3] + v(0.0, 0.58, -0.05)
}

/// The boys answering (their `n`th line): over the old man's shoulder onto the three, or low on
/// the one who speaks, in turn.
fn boys(t: f32, n: usize) -> Take {
    let face = speaker_face(n);
    let (k, dof) = if n.is_multiple_of(2) {
        // Over his shoulder, looking down on them as he does, his shoulder soft.
        (key(v(1.08, 1.3, -2.72), BOYS_MID.lerp(face, 0.5) + v(0.2, 0.25, 0.0), 25.0), SHALLOW.and_front(1.0, 2.5))
    } else {
        // Low, on the one who speaks, his friends beside him.
        (key(face + v(0.22, -0.08, -1.25), face + v(0.0, -0.08, 0.0), LONG), SHALLOW)
    };
    let push = ease(0.0, 6.0, t);
    let k = Key { eye: k.eye + (k.look - k.eye) * 0.06 * push, ..k };
    take(k, face, dof)
}

/// The souls rising as he names them: low, looking up past the boys at them over the wet flags
/// (from the side of the three, where none of them stands between us and a soul).
fn souls(t: f32) -> Take {
    let u = ease(0.0, 10.0, t);
    let k = key(v(-1.4, 0.27, 2.6), v(-0.08, 0.98, -2.7), 44.0).to(key(v(-1.3, 0.29, 2.35), v(-0.08, 1.04, -2.7), 41.0), u);
    take(k, SOUL_AT[1] + v(0.0, 1.0, 0.0), MEDIUM.and_front(0.8, 2.2))
}

/// His promise: closer and slower than anything before it.
fn promise(t: f32) -> Take {
    let u = ease(0.0, 14.0, t);
    let face = old_face();
    let k = key(v(0.9, 1.0, -0.6), face + v(0.0, -0.15, 0.0), PORTRAIT + 3.0)
        .to(key(v(0.97, 1.02, -0.8), face + v(0.0, -0.14, 0.0), PORTRAIT + 1.0), u);
    take(k, face, SHALLOW)
}

/// The end: close behind the boys as the souls go into them; pulling back as the lamps redden
/// and the door opens on the stair's red light; holding wide as they walk into it; and in on the
/// door when they are gone.
fn descent(t: f32) -> Take {
    let back = ease(0.6, stage::DOOR + 2.4, t);
    let home = ease(9.5, 17.0, t);
    let close = key(v(0.12, 0.92, 3.05), v(0.0, 0.8, -3.4), 42.0);
    // (Low enough that the boys walking away stay in the picture under the lamps.)
    let wide = key(v(0.15, 0.95, 3.7), v(0.0, 1.22, -3.4), 52.0);
    let door = key(v(0.05, 1.12, 1.25), v(0.0, 1.0, -3.6), 42.0);
    let k = close.to(wide, back).to(door, home);
    let boys = BOYS_MID + v(0.0, 0.5, 0.0);
    let focus = boys.lerp(v(0.0, 1.0, -3.4), smooth(stage::DOOR, stage::WALK + 3.0, t));
    take(k, focus, lerp_dof(MEDIUM, DEEP, back * (1.0 - home)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::logic::story::{first, SCENE_ONE};

    /// The moments a test walks: each action beat through its whole hold, each line through its
    /// first `talk` seconds (a line waits for the player, its shot held), every `step` seconds.
    fn moments(step: f32, talk: f32) -> Vec<(usize, f32)> {
        let mut out = Vec::new();
        for (i, b) in SCENE_ONE.iter().enumerate() {
            let end = if b.waits() { talk } else { b.hold as f32 - 0.01 };
            let mut t = 0.0;
            while t <= end {
                out.push((i, t));
                t += step;
            }
        }
        out
    }

    fn view_at(beat: usize, t: f32) -> StoryView {
        StoryView { scene: Scene::One, beat, t, age: Scene::One.time_at(beat, t as f64) as f32, shown: 1e3, souls: 0b111 }
    }

    /// The cuts inside a beat (seconds into it): everywhere else the camera must move smoothly.
    fn cuts(shot: Shot) -> Vec<f32> {
        match shot {
            Shot::Establish => vec![stage::MISKICK + 0.35, stage::BALL_AT_GATE],
            Shot::Taken => vec![stage::RUSH, stage::GRAB - 0.02, stage::TURN + 0.25],
            Shot::Arrive => vec![stage::REVEAL],
            _ => vec![],
        }
    }

    #[test]
    fn every_shot_moves_smoothly_between_its_cuts() {
        let dt = 1.0 / 120.0;
        for (i, b) in SCENE_ONE.iter().enumerate() {
            let end = if b.waits() { 20.0 } else { b.hold as f32 };
            let cut = cuts(b.shot);
            let mut was = scene_one(view_at(i, 0.0));
            let mut t = dt;
            while t < end {
                let now = scene_one(view_at(i, t));
                if !cut.iter().any(|&c| (t - dt..t + 1e-4).contains(&c)) {
                    let speed = now.eye.distance(was.eye) / dt;
                    let turn = (now.look - now.eye).normalize().angle_between((was.look - was.eye).normalize()) / dt;
                    let zoom = (now.lens.fov - was.lens.fov).abs() / dt;
                    let at = format!("{:?} (beat {i}) at {t:.3} s", b.shot);
                    assert!(speed < 6.0, "{at}: the camera jumps ({speed:.1} m/s)");
                    assert!(turn < 3.0, "{at}: the camera whips round ({turn:.2} rad/s)");
                    assert!(zoom < 25.0, "{at}: the lens snaps ({zoom:.1} degrees/s)");
                    // (Krampus's rush is fast, and the focus rides on him.)
                    assert!((now.focus - was.focus).length() / dt < 40.0, "{at}: the focus jumps");
                }
                was = now;
                t += dt;
            }
        }
        // Scene I runs on from the title card into the crane without a cut.
        let end = scene_one(view_at(0, SCENE_ONE[0].hold as f32 - 1e-3));
        let start = scene_one(view_at(1, 0.0));
        assert!(end.eye.distance(start.eye) < 0.05);
    }

    /// The distance from `p` to a box drawn as a unit cube by `m` (scale, rotation, translation).
    fn box_distance(m: Mat4, p: Vec3) -> f32 {
        let (s, r, t) = m.to_scale_rotation_translation();
        let q = r.inverse() * (p - t);
        let half = s.abs() * 0.5;
        (q - q.clamp(-half, half)).length()
    }

    /// Is `p` within `margin` of rig draw `d`'s parts (each part's voxels' bounding box)?
    fn in_rig(list: &RigList, d: usize, p: Vec3, margin: f32) -> bool {
        let draw = &list.draws[d];
        let mut boxes: HashMap<u32, (Vec3, Vec3)> = HashMap::new();
        for f in &list.meshes[d].faces {
            let part = f.part_face & 0xff;
            let e = boxes.entry(part).or_insert((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)));
            e.0 = e.0.min(Vec3::from(f.pos));
            e.1 = e.1.max(Vec3::from(f.pos));
        }
        boxes.into_iter().any(|(part, (lo, hi))| {
            let m = Mat4::from_cols_array(&list.parts[(draw.part_base + part) as usize]);
            let q = m.inverse().transform_point3(p);
            let pad = draw.size * 0.5 + margin;
            q.cmpge(lo - Vec3::splat(pad)).all() && q.cmple(hi + Vec3::splat(pad)).all()
        })
    }

    /// RENDERING.md, "Scene I's camera": no frame of any shot puts the lens inside the set or
    /// inside one of the cast.
    #[test]
    fn the_camera_never_stands_inside_the_sets_or_the_cast() {
        // (Just past the near plane's corners.)
        const CLEAR: f32 = 0.13;
        let mut game = Game::new();
        let souls = first(Shot::Souls).unwrap();
        let mut faults = Vec::new();
        for (beat, t) in moments(0.05, 14.0) {
            game.story_jump(beat, t as f64);
            let sv = game.story.as_ref().unwrap().view();
            game.view.story = Some(sv);
            game.view.story_rise = [if beat >= souls { 1.0 } else { 0.0 }; 3];
            let mut f = Frame::default();
            game.view.emit(&game.scene(), &mut f);
            let eye = game.view.story_camera(sv).eye;
            if (beat, t) == (first(Shot::OldMan).unwrap(), 0.0) {
                // (The test sees what it looks for: the old man's middle, and the back wall.)
                let him = GATEWAY + OLD_MAN_AT + v(0.0, 0.6, 0.0);
                assert!((0..f.rigs.draws.len()).any(|d| in_rig(&f.rigs, d, him, 0.0)));
                let wall = GATEWAY + v(-3.0, 2.0, -3.6);
                assert!(f.sets.iter().flat_map(|s| s.instances.iter()).any(|i| box_distance(Mat4::from_cols_array(&i.m), wall) == 0.0));
            }
            let origin = if sv.shot().place() == crate::logic::story::Place::Graveyard { GRAVEYARD } else { GATEWAY };
            let at = format!("{:?} (beat {beat}) at {t:.2} s, the eye at {:.2}", sv.shot(), eye - origin);
            let solid = f.sets.iter().flat_map(|s| s.instances.iter()).chain(&f.opaque[MeshKind::Cube as usize]);
            for i in solid {
                let m = Mat4::from_cols_array(&i.m);
                let size = m.to_scale_rotation_translation().0.abs();
                // (Leaves and grit blowing past are not scenery.)
                if size.max_element() < 0.1 || m.w_axis.truncate().distance(eye) > size.length() + CLEAR {
                    continue;
                }
                if box_distance(m, eye) < CLEAR {
                    faults.push(format!("{at}: in the scenery (a block {size:.2} at {:.2})", m.w_axis.truncate() - origin));
                    break;
                }
            }
            for d in 0..f.rigs.draws.len() {
                if in_rig(&f.rigs, d, eye, CLEAR) {
                    faults.push(format!("{at}: inside one of the cast"));
                }
            }
        }
        assert!(faults.is_empty(), "{} frames:\n{}", faults.len(), faults.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }

    /// Where `p` is on the screen (0..1 across, 0..1 down the whole screen, the letterbox
    /// included), if it is in front of the camera.
    fn on_screen(cam: &StoryCam, p: Vec3) -> Option<Vec2> {
        let proj = Mat4::perspective_rh(cam.lens.fov.to_radians(), 16.0 / 9.0, NEAR, FAR) * Mat4::look_at_rh(cam.eye, cam.look, Vec3::Y);
        let c = proj * p.extend(1.0);
        (c.w > 0.0).then(|| Vec2::new(c.x / c.w * 0.5 + 0.5, 0.5 - c.y / c.w * 0.5))
    }

    /// The one who speaks keeps his face in the picture, clear of the letterbox's bars and above
    /// the dialogue box at the foot of the screen (`hud::story`).
    #[test]
    fn whoever_speaks_is_framed_above_the_dialogue_box() {
        for (beat, t) in moments(0.25, 14.0) {
            let b = SCENE_ONE[beat];
            let sv = view_at(beat, t);
            let face = match (b.shot, b.speaker) {
                // (The souls are the subject while he names them; and the old man is found by his
                // lantern first.)
                (Shot::Souls, _) => continue,
                (Shot::Easy, _) if t < 3.6 => continue,
                (_, Speaker::OldMan) => GATEWAY + old_face(),
                (Shot::Scream, _) => GRAVEYARD + v(-0.45, 0.6, 2.6),
                (_, Speaker::Boys) => GATEWAY + speaker_face(said_before(sv, Speaker::Boys)),
                (_, Speaker::Dog) => GATEWAY + DOG_LUNGE + (BOYS_MID - DOG_LUNGE).normalize() * 0.62 + v(0.0, 0.62, 0.0),
                (_, Speaker::Narrator) => continue,
            };
            let cam = scene_one(sv);
            let at = format!("{:?} (beat {beat}) at {t:.2} s", b.shot);
            let p = on_screen(&cam, face).unwrap_or_else(|| panic!("{at}: the speaker is behind the camera"));
            assert!((0.06..0.94).contains(&p.x), "{at}: the speaker's face is off the side of the picture ({p})");
            assert!((0.12..0.76).contains(&p.y), "{at}: the speaker's face is under the bars or the dialogue box ({p})");
        }
    }
}
