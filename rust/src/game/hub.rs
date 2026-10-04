//! The Portals of Hell in play (the layout is `logic::hub`). No foes come
//! here. A portal's doors swing open as he comes up to them, showing the
//! swirl of its sin's realm, and pushing through them takes him into the
//! realm (`game::realm`); a realm he has cleared keeps its doors shut with
//! its key in the lock. With all seven keys the seal breaks: the ground
//! shakes, the wall of fire round the volcano dies and four causeways rise
//! out of the lava to the House of the Devil.

use super::{Banner, Game};
use crate::audio::Sfx;
use crate::logic::hub::{self, Gate};
use crate::logic::sins::ALL_SINS;
use crate::logic::types::{dist_sq, Tile, V};

/// Seconds for a portal's doors to swing fully open.
const DOOR_SECONDS: f64 = 0.9;
/// He must come this close to a gate's threshold for its doors to open.
const DOOR_REACH: f64 = 2.2;
/// Seconds for the causeways to rise once the seal breaks.
pub const SEAL_SECONDS: f64 = 5.0;

/// The hub while he is in it.
#[derive(Clone, Debug)]
pub struct HubState {
    pub gates: [Gate; 7],
    /// How far each portal's doors stand open, 0 to 1.
    pub doors: [f64; 7],
    /// How far the causeways have risen, 0 to 1.
    pub seal: f64,
    /// Seconds before a word at a door or the House is said again.
    hint: f64,
}

impl HubState {
    fn new(sealed: bool) -> Self {
        Self { gates: hub::gates(), doors: [0.0; 7], seal: if sealed { 0.0 } else { 1.0 }, hint: 0.0 }
    }

    /// The causeways are rising now.
    pub fn rising(&self) -> bool {
        self.seal > 0.0 && self.seal < 1.0
    }
}

impl Game {
    /// Into the Portals of Hell. With every sin's key already held, the causeways stand risen.
    pub fn load_hub(&mut self) {
        self.load(hub::portals_of_hell(), 1);
    }

    /// Called as the hub level loads.
    pub(super) fn open_hub(&mut self) {
        let broken = self.sins_cleared & ALL_SINS == ALL_SINS;
        self.hub = Some(HubState::new(!broken));
        if broken {
            self.raise_causeways();
        }
    }

    /// The causeways' tiles become ground he can walk.
    fn raise_causeways(&mut self) {
        for c in hub::causeways() {
            for p in c.tiles() {
                self.level.grid.set(p.x, p.y, Tile::Open);
            }
        }
    }

    pub(super) fn update_hub(&mut self, dt: f64) {
        let Some(mut h) = self.hub.take() else { return };
        h.hint = (h.hint - dt).max(0.0);
        let hp = self.hero.pos();
        for i in 0..7 {
            let gate = h.gates[i];
            let front = gate.front();
            let near = self.hero.alive && dist_sq(hp, front.v()) <= DOOR_REACH * DOOR_REACH;
            let cleared = self.sins_cleared & gate.sin.bit() != 0;
            let was = h.doors[i];
            let want = if near && !cleared { 1.0 } else { 0.0 };
            h.doors[i] = if want > was { (was + dt / DOOR_SECONDS).min(1.0) } else { (was - dt / DOOR_SECONDS).max(0.0) };
            if was == 0.0 && h.doors[i] > 0.0 {
                self.audio.play(Sfx::DoorGrind);
            }
            if near && cleared && h.hint <= 0.0 {
                h.hint = 3.0;
                self.floater(
                    front.x as f64,
                    front.y as f64 - 0.8,
                    format!("{} IS CONQUERED", gate.sin.name().to_uppercase()),
                    1.6,
                    Some(gate.sin.color()),
                );
            }
            // Through the open doors: into the realm beyond.
            if h.doors[i] >= 1.0 && near && self.hero.occupied() == front && self.facing_gate(gate) {
                self.view.shake(0.3);
                self.view.sparkle(&self.level.grid, gate.tile.x as f64, gate.tile.y as f64, 1.2, gate.sin.color(), 40);
                self.enter_realm(gate.sin);
                return;
            }
        }
        // The seal of the seven: every key held, and it breaks.
        if self.sins_cleared & ALL_SINS == ALL_SINS && h.seal < 1.0 {
            if h.seal == 0.0 {
                self.audio.play(Sfx::BossSlam);
                self.audio.play(Sfx::DoorGrind);
                self.juice.banner = Some(Banner {
                    text: "THE SEAL OF THE SEVEN IS BROKEN".into(),
                    sub: "THE WAY TO THE HOUSE OF THE DEVIL RISES FROM THE LAVA".into(),
                    color: 0xff5a2a,
                    life: 4.0,
                    max: 4.0,
                });
            }
            h.seal = (h.seal + dt / SEAL_SECONDS).min(1.0);
            self.view.shake(0.08);
            if h.seal >= 1.0 {
                self.raise_causeways();
                self.audio.play(Sfx::Whoosh);
            }
        }
        // At the House's door, once the way is open.
        let door = hub::house_door();
        let step = V::new(door.x as f64, door.y as f64 + 1.0);
        if h.seal >= 1.0 && dist_sq(hp, step) < 2.0 && h.hint <= 0.0 {
            h.hint = 4.0;
            self.audio.play(Sfx::Denied);
            self.juice.banner = Some(Banner {
                text: "THE HOUSE OF THE DEVIL".into(),
                sub: "NO ONE HAS SEEN INSIDE   ITS DOORS OPEN LATER".into(),
                color: 0xff3a2a,
                life: 3.0,
                max: 3.0,
            });
        }
        self.hub = Some(h);
    }

    /// Is he facing the gate (pushing into its doors)?
    fn facing_gate(&self, gate: Gate) -> bool {
        let to = V::new((gate.tile.x - gate.front().x) as f64, (gate.tile.y - gate.front().y) as f64);
        let aim = self.hero.aim();
        aim.x * to.x + aim.y * to.y > 0.5
    }

    /// Where the camera looks while the causeways rise: at the volcano.
    pub(super) fn hub_focus(&self) -> Option<glam::Vec3> {
        let h = self.hub.as_ref()?;
        let (cx, cy) = hub::volcano().center();
        h.rising().then(|| glam::Vec3::new(cx, 0.0, cy))
    }
}
