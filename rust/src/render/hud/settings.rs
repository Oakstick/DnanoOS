//! The settings page on screen (UI.md §8; the page's rules are `game::settings_page`): a panel
//! over the title or the pause menu, the five tabs across its top, the highlighted tab's rows
//! (each a label and its value: a switch, a slider, a choice between arrows, or a binding's two
//! slots and its reset), a line on the highlighted row, the prompts, and Back. Rebinding shows the
//! slot listening (its time running out) and, when the control is another action's, the question.
//! Its layout is a function of the window alone, so the game hit-tests what it draws.

use super::*;
use crate::game::settings_page::{rows, Kind, Row, Tab, CONFLICT_CHOICES, LISTEN_SECONDS};
use crate::input::{Control, Device};
use crate::render::overlay::Style;

/// Where everything on the page sits, in logical pixels.
pub struct Layout {
    pub panel: [f32; 4],
    pub tabs: [[f32; 4]; 5],
    /// The rows shown: each one's index in its tab and its rectangle.
    pub rows: Vec<(usize, [f32; 4])>,
    /// Where the list's rows begin and the rows' height.
    pub list: [f32; 4],
    pub row_h: f32,
    pub back: [f32; 4],
    /// The help line's and the note's heights.
    pub help_y: f32,
    pub note_y: f32,
}

/// The panel's rectangle.
fn panel_rect(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let pw = (w - t * 2.0).min(t * 46.0);
    let (top, bottom) = (t * 1.0, h - t * 1.0);
    [(w - pw) / 2.0, top, pw, bottom - top]
}

/// The list's rectangle and its rows' height.
fn list_rect(w: f32, h: f32) -> ([f32; 4], f32) {
    let t = tile_for(w, h);
    let [px, py, pw, ph] = panel_rect(w, h);
    let top = py + t * 4.6;
    let bottom = py + ph - t * 5.0;
    ([px + t * 0.8, top, pw - t * 1.6, (bottom - top).max(t)], t * 1.32)
}

/// How many rows the list shows at once.
pub fn visible_rows(w: f32, h: f32) -> usize {
    let (list, row_h) = list_rect(w, h);
    ((list[3] / row_h).floor() as usize).max(1)
}

/// The page's layout for a tab of `n` rows, from row `scroll` down.
pub fn layout(w: f32, h: f32, n: usize, scroll: usize) -> Layout {
    let t = tile_for(w, h);
    let panel = panel_rect(w, h);
    let [px, py, pw, ph] = panel;
    let gap = t * 0.3;
    let tw = (pw - t * 1.6 - gap * 4.0) / 5.0;
    let ty = py + t * 2.5;
    let tabs = std::array::from_fn(|k| [px + t * 0.8 + k as f32 * (tw + gap), ty, tw, t * 1.25]);
    let (list, row_h) = list_rect(w, h);
    let shown = visible_rows(w, h);
    let rows =
        (scroll..n.min(scroll + shown)).map(|i| (i, [list[0], list[1] + (i - scroll) as f32 * row_h, list[2], row_h * 0.9])).collect();
    let back = [px + pw / 2.0 - t * 3.0, py + ph - t * 1.75, t * 6.0, t * 1.2];
    Layout { panel, tabs, rows, list, row_h, back, help_y: py + ph - t * 4.3, note_y: py + ph - t * 3.4 }
}

impl Layout {
    fn row_rect(&self, i: usize) -> Option<[f32; 4]> {
        self.rows.iter().find(|(k, _)| *k == i).map(|(_, r)| *r)
    }

    /// The value's part of row `i`: from a little under half way along to its end.
    fn value(&self, i: usize) -> [f32; 4] {
        let r = self.row_rect(i).unwrap_or([0.0; 4]);
        let x = r[0] + r[2] * 0.5;
        [x, r[1], r[0] + r[2] - x - r[3] * 0.3, r[3]]
    }

    /// Row `i`'s three cells across its value (a binding's two slots and its reset).
    pub fn cells(&self, i: usize) -> [[f32; 4]; 3] {
        let [x, y, w, h] = self.value(i);
        [[x, y, w * 0.39, h], [x + w * 0.41, y, w * 0.39, h], [x + w * 0.82, y, w * 0.18, h]]
    }

    /// A slider's track in row `i`.
    pub fn track(&self, i: usize) -> [f32; 4] {
        let [x, y, w, h] = self.value(i);
        [x + h * 0.3, y + h * 0.42, w * 0.74 - h * 0.3, h * 0.16]
    }
}

/// The conflict question's three answers (swap, take it, cancel).
pub fn conflict_rects(w: f32, h: f32) -> [[f32; 4]; 3] {
    let t = tile_for(w, h);
    let [x, y, pw, ph] = conflict_panel(w, h);
    let (bw, bh, gap) = ((pw - t * 2.4) / 3.0, t * 1.3, t * 0.4);
    std::array::from_fn(|k| [x + t * 0.8 + k as f32 * (bw + gap), y + ph - bh - t * 0.8, bw, bh])
}

fn conflict_panel(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let (pw, ph) = ((t * 28.0).min(w - t * 2.0), t * 8.6);
    [(w - pw) / 2.0, (h - ph) / 2.0, pw, ph]
}

/// A small triangle pointing left or right (a choice's arrows).
fn arrow(o: &mut Overlay, x: f32, y: f32, s: f32, right: bool, color: [f32; 4]) {
    let d = if right { s } else { -s };
    o.triangle([x - d * 0.5, y - s * 0.6], [x - d * 0.5, y + s * 0.6], [x + d * 0.5, y], color);
}

/// A control's glyph centred in a cell, or a dash for an empty slot.
fn slot(o: &mut Overlay, c: Option<Control>, device: Device, cell: [f32; 4], size: f32, alpha: f32) {
    let (cx, cy) = (cell[0] + cell[2] / 2.0, cell[1] + cell[3] / 2.0);
    match c {
        Some(c) => {
            let gw = glyphs::control_width(o, c, device, size);
            if gw <= cell[2] - size * 0.3 {
                glyphs::draw_control(o, c, device, cx - gw / 2.0, cy, size);
            } else {
                // Too wide for the cell (a long key name): its name, as words.
                let s = Style::body(size * 0.55, rgba(TEXT, alpha)).centered();
                o.text_styled(&crate::input::control_name(c, device), cx, cy, &s);
            }
        }
        None => {
            o.text_styled("-", cx, cy, &Style::body(size * 0.6, rgba(TEXT_DIM, alpha)).centered());
        }
    }
}

/// The page, over whatever is behind it.
pub fn draw_settings(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let page = &g.settings_page;
    let list = rows(page.tab);
    let n = list.len();
    let l = layout(w, h, n, page.scroll);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.72]);
    let [px, py, pw, _] = l.panel;
    // The glass laid twice (the pause menu must not read through it), its copper rule, the title
    // in the display serif over a divider.
    let r = l.panel;
    style::shadow(o, r, t, 1.0);
    style::glass(o, r, t, 1.0);
    style::glass(o, r, t, 0.85);
    style::frame(o, r, t, rgba(COPPER, 0.8));
    o.text_styled("SETTINGS", px + pw / 2.0, py + t * 1.35, &style::name(t, 1.25, rgba(GOLD, 1.0)).centered());
    style::divider(o, px + pw * 0.3, py + t * 2.05, pw * 0.4, t, rgba(COPPER, 0.8));

    // ---- the tabs, the open one lit, LB and RB at their ends
    for (k, (r, tab)) in l.tabs.iter().zip(Tab::ALL).enumerate() {
        let open = tab == page.tab;
        o.rect(r[0], r[1], r[2], r[3], if open { rgba(COPPER, 0.32) } else { [1.0, 1.0, 1.0, 0.04] });
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.06).max(1.0), if open { rgba(GOLD, pulse) } else { rgba(COPPER, 0.45) });
        let s = Style::body(t * 0.5, rgba(if open { GOLD } else { TEXT_DIM }, 1.0)).bold().centered().tracking(0.1);
        o.text_styled(tab.name(), r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, &s);
        let _ = k;
    }
    let first = l.tabs[0];
    let last = l.tabs[4];
    let gh = t * 0.75;
    glyphs::draw_prompt_right(o, &g.input, Action::TabPrev, first[0] - t * 0.15, first[1] + first[3] / 2.0, gh);
    glyphs::draw_prompt(o, &g.input, Action::TabNext, last[0] + last[2] + t * 0.15, last[1] + last[3] / 2.0, gh);

    // ---- the rows
    let label = Style::body(t * 0.64, rgba(TEXT, 1.0));
    let value = Style::body(t * 0.62, rgba(SAND, 1.0));
    let device = if page.pad { g.names_device() } else { Device::KeyboardMouse };
    for &(i, r) in &l.rows {
        let row = list[i];
        let lit = i == page.row && page.conflict.is_none();
        let cy = r[1] + r[3] / 2.0;
        if lit {
            o.rect(r[0], r[1], r[2], r[3], rgba(COPPER, 0.22));
            o.rect(r[0], r[1], t * 0.12, r[3], rgba(GOLD, pulse));
        } else if i % 2 == 0 {
            o.rect(r[0], r[1], r[2], r[3], [1.0, 1.0, 1.0, 0.025]);
        }
        // A rule over the bindings, which begin after the row choosing their side.
        if matches!(row, Row::Device) {
            o.rect(r[0], r[1] - t * 0.08, r[2], (t * 0.04).max(1.0), rgba(COPPER, 0.5));
        }
        let lc = if lit { GOLD } else { TEXT };
        o.text_styled(&row.label(), r[0] + t * 0.5, cy, &label.color(rgba(lc, 1.0)));
        let v = l.value(i);
        match row.kind() {
            Kind::Toggle => {
                let on = g.settings_value(row) == "On";
                let (sw, sh) = (t * 1.6, t * 0.7);
                let (sx, sy) = (v[0] + t * 0.3, cy - sh / 2.0);
                super::legend::pill(o, sx, sy, sw, sh, if on { rgba(0xc89a50, 1.0) } else { rgba(0x3a3f48, 1.0) });
                let kx = if on { sx + sw - sh / 2.0 } else { sx + sh / 2.0 };
                o.circle(kx, cy, sh * 0.38, if on { rgba(0x1a1612, 1.0) } else { rgba(TEXT_DIM, 1.0) });
                o.text_styled(&g.settings_value(row), sx + sw + t * 0.5, cy, &value.color(rgba(if on { GOLD } else { TEXT_DIM }, 1.0)));
            }
            Kind::Slider(_) => {
                let tr = l.track(i);
                let frac = g.settings_slider(row).map(|(rg, x)| rg.frac(x)).unwrap_or(0.0);
                glyphs::rounded_rect(o, tr[0], tr[1], tr[2], tr[3], tr[3] / 2.0, [1.0, 1.0, 1.0, 0.14]);
                glyphs::rounded_rect(
                    o,
                    tr[0],
                    tr[1],
                    (tr[2] * frac).max(tr[3]),
                    tr[3],
                    tr[3] / 2.0,
                    rgba(if lit { GOLD } else { COPPER }, 1.0),
                );
                o.circle(tr[0] + tr[2] * frac, tr[1] + tr[3] / 2.0, tr[3] * 1.5, rgba(if lit { 0xfff2d8 } else { SAND }, 1.0));
                o.text_styled(&g.settings_value(row), v[0] + v[2], cy, &value.right());
            }
            Kind::Choice => {
                let text = g.settings_value(row);
                let mid = v[0] + v[2] * 0.42;
                let a = if lit { rgba(GOLD, 1.0) } else { rgba(TEXT_DIM, 0.8) };
                arrow(o, v[0] + t * 0.5, cy, t * 0.32, false, a);
                arrow(o, v[0] + v[2] * 0.84, cy, t * 0.32, true, a);
                o.text_styled(&text, mid, cy, &value.centered().color(rgba(if lit { 0xfff2d8 } else { SAND }, 1.0)));
            }
            Kind::Binding => {
                let Row::Bind(action) = row else { continue };
                let side = g.input.bindings.side(action, page.pad);
                let cells = l.cells(i);
                for (k, cell) in cells.iter().enumerate() {
                    let chosen = lit && page.column == k;
                    let listening = page.listen.is_some_and(|ls| ls.action == action && ls.slot == k);
                    let inset = t * 0.08;
                    let c = [cell[0] + inset, cell[1] + inset, cell[2] - 2.0 * inset, cell[3] - 2.0 * inset];
                    o.rect(c[0], c[1], c[2], c[3], [0.0, 0.0, 0.0, 0.35]);
                    let edge = if listening {
                        rgba(0xfff2d8, pulse)
                    } else if chosen {
                        rgba(GOLD, pulse)
                    } else {
                        [1.0, 1.0, 1.0, 0.1]
                    };
                    o.rect_outline(c[0], c[1], c[2], c[3], (t * 0.05).max(1.0), edge);
                    if k == 2 {
                        let s = Style::body(t * 0.48, rgba(if chosen { GOLD } else { TEXT_DIM }, 1.0)).centered();
                        o.text_styled("RESET", c[0] + c[2] / 2.0, cy, &s);
                    } else if listening {
                        let s = Style::body(t * 0.5, rgba(0xfff2d8, pulse)).centered();
                        o.text_styled(if page.pad { "Press a button" } else { "Press a key" }, c[0] + c[2] / 2.0, cy - t * 0.08, &s);
                        // Its time running out along the slot's foot.
                        let left = page.listen.map(|ls| 1.0 - ((g.time - ls.since) / LISTEN_SECONDS) as f32).unwrap_or(0.0).clamp(0.0, 1.0);
                        o.rect(c[0], c[1] + c[3] - t * 0.1, c[2] * left, t * 0.1, rgba(GOLD, 0.9));
                    } else {
                        slot(o, side.get(k).copied(), device, c, c[3] * 0.84, 1.0);
                    }
                }
            }
            Kind::Button => {
                let armed = page.reset_armed;
                let s = Style::body(
                    t * 0.55,
                    rgba(
                        if armed {
                            0xff8a6a
                        } else if lit {
                            GOLD
                        } else {
                            SAND
                        },
                        1.0,
                    ),
                )
                .bold()
                .centered();
                let b = [v[0], r[1] + t * 0.1, v[2] * 0.8, r[3] - t * 0.2];
                o.rect_outline(b[0], b[1], b[2], b[3], (t * 0.05).max(1.0), if armed { rgba(0xff6a4a, pulse) } else { rgba(COPPER, 0.7) });
                o.text_styled(if armed { "CHOOSE AGAIN TO RESET" } else { "RESET ALL" }, b[0] + b[2] / 2.0, cy, &s);
            }
        }
    }
    // A long list's place in it.
    let shown = l.rows.len();
    if n > shown {
        let [lx, ly, lw, lh] = l.list;
        let x = lx + lw + t * 0.25;
        o.rect(x, ly, t * 0.1, lh, [1.0, 1.0, 1.0, 0.08]);
        let (a, b) = (page.scroll as f32 / n as f32, (page.scroll + shown) as f32 / n as f32);
        o.rect(x, ly + lh * a, t * 0.1, lh * (b - a), rgba(COPPER, 0.9));
    }

    // ---- the highlighted row's line, a note, the prompts, Back
    let help = Style::body(t * 0.56 * prefs().prose.min(1.1), rgba(TEXT_DIM, 1.0)).centered();
    if let Some(row) = g.settings_row() {
        glyphs::prompt_text(o, &g.input, row.help(), px + pw / 2.0, l.help_y, &help);
    }
    if let Some((note, _)) = &page.note {
        let s = Style::body(t * 0.6, rgba(GOLD, 1.0)).bold().centered();
        o.text_styled(note, px + pw / 2.0, l.note_y, &s);
    }
    let py_prompts = l.back[1] - t * 0.55;
    if page.listen.is_some() {
        // The two that cancel are fixed (Esc, and the Menu button), so drawn as they are.
        let s = Style::body(t * 0.48, rgba(TEXT_DIM, 1.0));
        let (a, b) = ("Press the control to bind     ", " or ");
        let gh = t * 0.48 * 1.3;
        let (esc, menu) = (Control::Key(winit::keyboard::KeyCode::Escape), Control::Pad(crate::input::PadButton::Menu));
        let pad_glyphs = g.names_device();
        let width = o.measure_styled(a, &s)
            + glyphs::control_width(o, esc, Device::KeyboardMouse, gh)
            + o.measure_styled(b, &s)
            + glyphs::control_width(o, menu, pad_glyphs, gh)
            + o.measure_styled(" cancel", &s);
        let mut pen = px + pw / 2.0 - width / 2.0;
        pen += o.text_styled(a, pen, py_prompts, &s);
        pen += glyphs::draw_control(o, esc, Device::KeyboardMouse, pen, py_prompts, gh);
        pen += o.text_styled(b, pen, py_prompts, &s);
        pen += glyphs::draw_control(o, menu, pad_glyphs, pen, py_prompts, gh);
        o.text_styled(" cancel", pen, py_prompts, &s);
    } else {
        let prompts = if matches!(g.settings_row(), Some(Row::Bind(_))) {
            "{Nav} choose     {Confirm} bind     {Secondary} clear     {Back} back"
        } else {
            "{Nav} choose     {NavLR} change     {Confirm} select     {Back} back"
        };
        glyphs::prompt_line(o, &g.input, prompts, px + pw / 2.0, py_prompts, t * 0.52, rgba(TEXT_DIM, 1.0), Align::Center);
    }
    let b = l.back;
    o.rect(b[0], b[1], b[2], b[3], [INK[0], INK[1], INK[2], 0.9]);
    o.rect_outline(b[0], b[1], b[2], b[3], (t * 0.07).max(1.0), rgba(COPPER, 0.8));
    o.text_styled("BACK", b[0] + b[2] / 2.0, b[1] + b[3] / 2.0, &Style::body(t * 0.55, rgba(GOLD, 1.0)).bold().centered());

    // ---- the conflict's question
    if let Some(c) = &page.conflict {
        o.rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.55]);
        let [x, y, cw, ch] = conflict_panel(w, h);
        let r = [x, y, cw, ch];
        style::shadow(o, r, t, 1.0);
        style::glass(o, r, t, 1.0);
        style::glass(o, r, t, 0.9);
        style::frame(o, r, t, rgba(GOLD, 0.7));
        let name = crate::input::control_name(c.control, device);
        let others: Vec<&str> = c.others.iter().map(|&a| crate::input::action_name(a)).collect();
        let head = Style::display(t * 0.75, rgba(GOLD, 1.0)).centered();
        o.text_styled("ALREADY BOUND", x + cw / 2.0, y + t * 1.0, &head);
        let gh = t * 1.0;
        let line = Style::body(t * 0.62, rgba(TEXT, 1.0));
        let words = format!("is {}'s. Give it to {}?", others.join(", "), crate::input::action_name(c.action));
        let gw = glyphs::control_width(o, c.control, device, gh);
        let ww = o.measure_styled(&words, &line);
        let x0 = x + cw / 2.0 - (gw + t * 0.3 + ww) / 2.0;
        glyphs::draw_control(o, c.control, device, x0, y + t * 2.35, gh);
        o.text_styled(&words, x0 + gw + t * 0.3, y + t * 2.35, &line);
        let old = g.input.bindings.side(c.action, g.settings_page.pad).get(c.slot).copied();
        let swap = match old {
            Some(o2) => format!("Swap: {} takes {}", others.join(", "), crate::input::control_name(o2, device)),
            None => format!("Swap: {} is left without it", others.join(", ")),
        };
        let lines = [swap, format!("Take it: {} loses {name}", others.join(", "))];
        let small = Style::body(t * 0.52, rgba(TEXT_DIM, 1.0)).centered();
        for (k, s) in lines.iter().enumerate() {
            o.text_styled(s, x + cw / 2.0, y + t * (3.55 + 0.75 * k as f32), &small);
        }
        for (k, (r, label)) in conflict_rects(w, h).iter().zip(CONFLICT_CHOICES).enumerate() {
            let chosen = k == c.choice;
            o.rect(r[0], r[1], r[2], r[3], if chosen { rgba(COPPER, 0.3) } else { [1.0, 1.0, 1.0, 0.04] });
            o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.07).max(1.0), if chosen { rgba(GOLD, pulse) } else { rgba(COPPER, 0.6) });
            let s = Style::body(t * 0.55, rgba(if chosen { GOLD } else { TEXT_DIM }, 1.0)).bold().centered();
            o.text_styled(label, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, &s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_fits_every_window_and_scales_with_the_ui() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (3840.0, 2160.0), (1024.0, 768.0)] {
            let l = layout(w, h, rows(Tab::Controls).len(), 0);
            let [px, py, pw, ph] = l.panel;
            assert!(px >= 0.0 && py >= 0.0 && px + pw <= w && py + ph <= h, "{w}x{h}");
            assert!(l.rows.len() >= 8, "{w}x{h}: {} rows", l.rows.len());
            for (_, r) in &l.rows {
                assert!(r[1] + r[3] <= l.help_y, "{w}x{h}: a row over the help line");
            }
            for t in l.tabs {
                assert!(t[0] >= px && t[0] + t[2] <= px + pw + 0.5);
            }
            // The Gameplay tab's few rows all show.
            assert_eq!(layout(w, h, rows(Tab::Gameplay).len(), 0).rows.len(), rows(Tab::Gameplay).len());
        }
        // At 150% the page is larger, and still on screen at 720p.
        let base = layout(1280.0, 720.0, 6, 0).tabs[0][3];
        style::set_ui_scale(1.5);
        let l = layout(1280.0, 720.0, 40, 0);
        assert!((l.tabs[0][3] / base - 1.5).abs() < 1e-3);
        assert!(l.panel[1] + l.panel[3] <= 720.0 && !l.rows.is_empty());
        style::set_ui_scale(1.0);
    }
}
