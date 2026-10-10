//! Hızır's lesson card (B04): at the top of the screen while he teaches, the step (1 of 5, its
//! name), what to press in the glyphs of the device used last, the last thing that went wrong, and
//! how to skip it. His words are in the panel (`dialogue`), in his voice.

use super::style::panel_ruled;
use super::*;
use crate::game::counsel::AMBER;
use crate::game::lesson::Step;

pub(super) fn draw(o: &mut Overlay, g: &Game, t: f32) {
    let Some(l) = g.lesson.as_ref() else { return };
    if g.screen != Screen::Playing {
        return;
    }
    let w = o.width;
    let fade = (l.age as f32 * 3.0).min(1.0);
    let (pw, ph) = (t * 19.0, t * 4.6);
    let r = [(w - pw) / 2.0, t * 1.1, pw, ph];
    panel_ruled(o, r, t, 0.92 * fade, AMBER);
    let cx = w / 2.0;
    let head = match l.step.number() {
        Some(n) => format!("HIZIR'S LESSON  {n}/{}   {}", Step::TAUGHT.len(), l.step.title()),
        None => l.step.title().to_string(),
    };
    let s = Style::display(t * 0.62, rgba(AMBER, fade)).centered().tracking(0.08).shadow(t * 0.05, t * 0.08);
    o.text_styled(&head, cx, r[1] + t * 0.95, &s);
    // The five steps as pips, the ones done lit.
    let n = Step::TAUGHT.len();
    let at = l.step.number().unwrap_or(if l.step == Step::Farewell { n + 1 } else { 0 });
    for k in 0..n {
        let x = cx + (k as f32 - (n as f32 - 1.0) / 2.0) * t * 0.7;
        let lit = k + 1 < at;
        let now = k + 1 == at;
        let c = if lit {
            rgba(AMBER, fade)
        } else if now {
            rgba(0xfff0d0, fade)
        } else {
            rgba(COPPER, 0.5 * fade)
        };
        style::diamond(o, x, r[1] + t * 1.55, t * if now { 0.17 } else { 0.12 }, c);
    }
    let body = Style::body(t * 0.52, rgba(0xf4ead8, fade)).centered();
    glyphs::prompt_text(o, &g.input, l.step.prompt(), cx, r[1] + t * 2.45, &body);
    if let Some(hint) = l.hint {
        let s = Style::body(t * 0.4, rgba(0xffc890, 0.9 * fade)).centered();
        o.text_styled(hint, cx, r[1] + t * 3.25, &s);
    }
    let dim = Style::body(t * 0.36, rgba(0xbfb4a0, 0.8 * fade)).centered();
    glyphs::prompt_text(o, &g.input, "{Pause} skip the lesson", cx, r[1] + t * 4.0, &dim);
}
