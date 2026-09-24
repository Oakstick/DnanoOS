//! Steamworks glue, compiled only with `--features steam`. The game runs the
//! same without it; when the Steam client is not running, init fails quietly.
//!
//! Achievement API names must match the ones defined on the Steamworks
//! partner site (App Admin > Stats & Achievements).

use crate::game::GameEvent;

pub const ACH_FIRST_NUGGET: &str = "ACH_FIRST_NUGGET";
pub const ACH_HARVEST: &str = "ACH_HARVEST";

pub struct Steam {
    client: steamworks::Client,
    single: steamworks::SingleClient,
}

impl Steam {
    pub fn init() -> Option<Self> {
        match steamworks::Client::init() {
            Ok((client, single)) => {
                client.user_stats().request_current_stats();
                Some(Self { client, single })
            }
            Err(e) => {
                eprintln!("steam unavailable: {e}");
                None
            }
        }
    }

    /// Pumps Steam callbacks and turns game events into achievements.
    pub fn frame(&mut self, events: &mut Vec<GameEvent>) {
        self.single.run_callbacks();
        let mut dirty = false;
        for e in events.iter() {
            let name = match e {
                GameEvent::FirstNugget => ACH_FIRST_NUGGET,
                GameEvent::Won => ACH_HARVEST,
            };
            if self.client.user_stats().achievement(name).set().is_ok() {
                dirty = true;
            }
        }
        if dirty {
            let _ = self.client.user_stats().store_stats();
        }
    }
}
