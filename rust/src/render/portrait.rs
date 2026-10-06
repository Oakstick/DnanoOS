//! Portraits (UI.md §6): a model of the game, posed and lit, drawn by the game's own renderer from
//! a portrait camera onto a clear background, into a slot of the UI atlas, for the HUD to draw as a
//! textured quad.
//!
//! The HUD asks with `Overlay::portrait(id, pose, ...)` as it draws; the book keeps which slot holds
//! which portrait and when it was made, and lists the ones due this frame (`due`): new ones, and
//! live ones whose refresh has come round. The app hands that list to `Gpu::render_portraits`
//! before the frame is drawn, so a portrait asked for is on screen the same frame. A slot not asked
//! for in a while is given to the next new one.
//!
//! ```ignore
//! // The old man speaking: remade every frame while his lips move, once a second otherwise.
//! let pose = Pose::talking(g.time as f32, line_age);
//! o.portrait(PortraitId::OldMan, pose, x, y, size, size, false, 1.0);
//! ```

use super::overlay::{portrait_slot, AtlasRegion, ALL_SLOTS, BIG_PORTRAIT, PORTRAIT_SIZE, PORTRAIT_SLOTS};
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::sins::Sin;

/// Whose portrait.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PortraitId {
    /// A hero, as he is played now (his weapon and outfit) if he is the one played.
    Hero(HeroKind),
    /// The old gatekeeper.
    OldMan,
    /// A foe of this kind.
    Foe(EnemyKind),
    /// The prince of this sin.
    Prince(Sin),
}

/// How the model stands for its portrait, and how often it is made again.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    /// Seconds of the model's idle (breathing, the head's slow turn).
    pub time: f32,
    /// Seconds into a line spoken, while he speaks.
    pub talking: Option<f32>,
    /// Seconds between makings: 0 makes it once, `EVERY_FRAME` every frame it is drawn.
    pub refresh: f32,
}

/// `Pose::refresh` for a portrait made every frame.
pub const EVERY_FRAME: f32 = -1.0;

impl Pose {
    /// Made once, at rest.
    pub fn still() -> Self {
        Self { time: 0.0, talking: None, refresh: 0.0 }
    }

    /// Alive: breathing at `time`, made again every `every` seconds.
    pub fn live(time: f32, every: f32) -> Self {
        Self { time, talking: None, refresh: every }
    }

    /// Speaking a line `since` seconds old: made every frame (his head and hand move with it).
    pub fn talking(time: f32, since: f32) -> Self {
        Self { time, talking: Some(since), refresh: EVERY_FRAME }
    }
}

/// A portrait to make this frame: its slot, whose, posed how, and how many pixels square.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Job {
    pub slot: usize,
    pub id: PortraitId,
    pub pose: Pose,
    pub size: u32,
}

impl Job {
    /// Where it goes in the atlas.
    pub fn region(&self) -> AtlasRegion {
        portrait_slot(self.slot).sub(0, 0, self.size, self.size)
    }
}

#[derive(Clone, Copy, Debug)]
struct Held {
    id: PortraitId,
    size: u32,
    /// The pose's time it was last made at (none: never yet).
    made: Option<f32>,
    /// The frame it was last asked for.
    used: u64,
}

/// Which slot holds which portrait (see the module's notes).
#[derive(Debug)]
pub struct PortraitBook {
    slots: [Option<Held>; ALL_SLOTS],
    frame: u64,
    /// The portraits to make this frame, for `Gpu::render_portraits`.
    pub due: Vec<Job>,
}

impl Default for PortraitBook {
    fn default() -> Self {
        Self { slots: [None; ALL_SLOTS], frame: 0, due: Vec::new() }
    }
}

/// The pixels a portrait asked for at `px` is made at: rounded up to a 16th, at most a big slot.
pub fn made_size(px: u32) -> u32 {
    px.clamp(16, BIG_PORTRAIT).div_ceil(16) * 16
}

/// The slots a portrait made at `size` may take: the small ones, or the big ones past their size.
fn pool(size: u32) -> std::ops::Range<usize> {
    if size > PORTRAIT_SIZE {
        PORTRAIT_SLOTS..ALL_SLOTS
    } else {
        0..PORTRAIT_SLOTS
    }
}

impl PortraitBook {
    /// A new frame: nothing is due yet.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.due.clear();
    }

    /// Asks for `id`'s portrait at `px` pixels square, posed: its slot and the pixels it is made at
    /// (the region `portrait_slot(slot).sub(0, 0, size, size)`), or none if every slot is in use
    /// this frame.
    pub fn request(&mut self, id: PortraitId, pose: Pose, px: u32) -> Option<(usize, u32)> {
        let size = made_size(px);
        let frame = self.frame;
        let k = match self.slots.iter().position(|s| s.is_some_and(|h| h.id == id && h.size == size)) {
            Some(k) => k,
            None => {
                // A free slot of its size, else the one asked for longest ago (never one in use
                // this frame).
                let range = pool(size);
                let k = range.clone().find(|&k| self.slots[k].is_none()).or_else(|| {
                    range.filter(|&k| self.slots[k].is_some_and(|h| h.used < frame)).min_by_key(|&k| self.slots[k].map(|h| h.used))
                })?;
                self.slots[k] = Some(Held { id, size, made: None, used: frame });
                k
            }
        };
        let held = self.slots[k].as_mut().expect("just found");
        held.used = frame;
        let due = match held.made {
            None => true,
            Some(_) if pose.refresh < 0.0 => true,
            Some(at) => pose.refresh > 0.0 && (pose.time - at >= pose.refresh || pose.time < at),
        };
        if due && !self.due.iter().any(|j| j.slot == k) {
            held.made = Some(pose.time);
            self.due.push(Job { slot: k, id, pose, size });
        }
        Some((k, size))
    }

    /// Forgets every portrait (a new level: the hero's gear or the models may have changed).
    pub fn forget(&mut self) {
        self.slots = [None; ALL_SLOTS];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_portrait_is_made_once_then_when_due() {
        let mut b = PortraitBook::default();
        b.begin_frame();
        let (k, size) = b.request(PortraitId::OldMan, Pose::still(), 100).unwrap();
        assert_eq!(size, 112);
        assert_eq!(b.due.len(), 1);
        b.begin_frame();
        assert_eq!(b.request(PortraitId::OldMan, Pose::still(), 100), Some((k, size)));
        assert!(b.due.is_empty(), "a still portrait is made once");
        // A live one: again once its refresh comes round.
        let hero = PortraitId::Hero(HeroKind::Ragnar);
        b.request(hero, Pose::live(0.0, 1.0), 64);
        assert_eq!(b.due.len(), 1);
        b.begin_frame();
        b.request(hero, Pose::live(0.5, 1.0), 64);
        assert!(b.due.is_empty());
        b.begin_frame();
        b.request(hero, Pose::live(1.2, 1.0), 64);
        assert_eq!(b.due.len(), 1);
        // Talking: every frame; asked twice in a frame, made once.
        b.begin_frame();
        b.request(PortraitId::OldMan, Pose::talking(3.0, 0.1), 100);
        b.request(PortraitId::OldMan, Pose::talking(3.0, 0.1), 100);
        assert_eq!(b.due.len(), 1);
    }

    #[test]
    fn slots_are_reused_longest_unused_first_and_never_while_in_use() {
        let mut b = PortraitBook::default();
        b.begin_frame();
        let kinds = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto];
        let mut n = 0;
        for size in (16..=PORTRAIT_SIZE).step_by(16) {
            for k in kinds {
                if n < PORTRAIT_SLOTS {
                    assert!(b.request(PortraitId::Hero(k), Pose::still(), size).is_some());
                    n += 1;
                }
            }
        }
        assert!(b.request(PortraitId::OldMan, Pose::still(), 64).is_none(), "all in use this frame");
        // (A large one has slots of its own.)
        let (big, size) = b.request(PortraitId::OldMan, Pose::still(), 392).unwrap();
        assert!(big >= PORTRAIT_SLOTS && size == 400 && portrait_slot(big).w == BIG_PORTRAIT);
        b.begin_frame();
        let (k, _) = b.request(PortraitId::OldMan, Pose::still(), 64).unwrap();
        assert_eq!(k, 0, "the first made is the longest unused");
        assert_eq!(b.due.len(), 1);
    }

    #[test]
    fn a_portrait_larger_than_a_small_slot_is_made_in_a_big_one() {
        let mut b = PortraitBook::default();
        b.begin_frame();
        assert_eq!(made_size(196), 208);
        assert_eq!(made_size(600), BIG_PORTRAIT);
        let (k, size) = b.request(PortraitId::OldMan, Pose::still(), 252).unwrap();
        assert!(k < PORTRAIT_SLOTS && size == 256);
        let (k, size) = b.request(PortraitId::OldMan, Pose::still(), 504).unwrap();
        assert!(k >= PORTRAIT_SLOTS && size == 512);
        assert_eq!(Job { slot: k, id: PortraitId::OldMan, pose: Pose::still(), size }.region(), portrait_slot(k));
    }
}
