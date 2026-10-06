//! The type specimen (`--capture out.png --specimen`): both faces at the sizes the HUD uses, plain,
//! bold, with the old drop shadow, outlined and glowing, the accented names, and the textured
//! quads (a picture turned, through a round mask, and as a 9-slice panel), for checking the type
//! and the overlay's quads in a capture. Not a screen of the game.

use super::*;
use crate::render::overlay::{Shadow, Style};
use crate::render::text::Family;

pub fn draw(o: &mut Overlay, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, rgba(0x14161c, 1.0));
    let mut y = t * 1.2;
    let x = t * 0.8;
    let sizes = [0.36, 0.42, 0.5, 0.6, 0.8, 1.0, 1.4];
    for family in [Family::Body, Family::Display] {
        for &k in &sizes {
            let size = t * k;
            let s = Style::new(family, size, rgba(TEXT, 1.0));
            let label = format!("{:.0}px", size);
            o.text_styled(&label, x, y, &Style::body(t * 0.36, rgba(TEXT_DIM, 1.0)));
            let mut cx = x + t * 2.2;
            cx += o.text_styled("The Garden of Krampus 0123", cx, y, &s) + t * 0.6;
            cx += o.text_styled("THE OLD MAN'S WORD", cx, y, &s.bold()) + t * 0.6;
            let shadow = Shadow { dx: 1.6, dy: 1.6, blur: 0.5, color: [0.0, 0.0, 0.0, 0.7] };
            cx += o.text_styled("Tülükuş Goryō", cx, y, &Style { shadow: Some(shadow), ..s.color(rgba(GOLD, 1.0)) }) + t * 0.6;
            o.text_styled("ARÇURA", cx, y, &s.outline((size * 0.08).max(1.0), rgba(0x000000, 1.0)).color(rgba(SAND, 1.0)));
            y += size * 1.25 + t * 0.15;
        }
        y += t * 0.3;
    }
    let s = Style::display(t * 1.6, rgba(GOLD, 1.0)).centered().glow(t * 0.25, rgba(0xff9a30, 0.55)).tracking(0.06);
    o.text_styled("ANCIENT BLADES", w * 0.5, y + t * 0.5, &s);
    // A wrapped paragraph in a measured box.
    let body = Style::body(t * 0.6, rgba(TEXT, 1.0));
    let (bx, by, bw) = (x, y + t * 1.6, t * 14.0);
    o.rect_outline(bx, by - t * 0.5, bw, t * 3.2, 1.0, rgba(COPPER, 0.8));
    o.wrap_styled(
        "Up, Ragnar. The bog slime strikes from afar; close the distance before it spits again, and mind the reeds.",
        bx,
        by,
        bw,
        t * 0.8,
        &body,
    );
    // The glyphs: every control on each device, and a line of prompts in each.
    use crate::input::{Control, Device, Input, MouseButton, PadButton};
    let mut gy = by + t * 3.4;
    let gh = t * 0.9;
    for d in [Device::KeyboardMouse, Device::Xbox, Device::PlayStation, Device::Generic] {
        let mut input = Input::new();
        input.set_last_device(d);
        let mut gx = x;
        let controls: Vec<Control> = if d.is_pad() {
            PadButton::ALL.iter().map(|&b| Control::Pad(b)).collect()
        } else {
            use winit::keyboard::KeyCode::*;
            vec![
                Control::Key(KeyW),
                Control::Key(Space),
                Control::Key(ShiftLeft),
                Control::Key(Escape),
                Control::Key(ArrowUp),
                Control::Mouse(MouseButton::Left),
                Control::Mouse(MouseButton::Right),
                Control::WheelUp,
                Control::WheelDown,
            ]
        };
        for c in controls {
            gx += glyphs::draw_control(o, c, d, gx, gy, gh) + t * 0.25;
        }
        let line = Style::body(t * 0.6, rgba(TEXT, 1.0));
        glyphs::prompt_text(
            o,
            &input,
            "{Move} run   {Attack} strike   {Block} shield   {Angel} archangel   {Confirm} take   {Back} leave",
            gx + t * 0.5,
            gy,
            &line,
        );
        gy += gh + t * 0.45;
    }
    // The quads: a picture turned, the same through a round window, and a 9-slice panel of it.
    let pic = crate::render::overlay::Picture::ViewTop.region();
    let (qx, qy) = (w - t * 9.0, y + t * 2.6);
    o.region_rotated(pic, qx, qy, t * 4.0, t * 2.25, 0.3, [1.0; 4]);
    let [u0, v0, u1, v1] = pic.uv();
    o.region_circle([(u0 + u1) / 2.0, (v0 + v1) / 2.0], (v1 - v0) / 2.0, 0.5, qx + t * 3.6, qy, t * 1.2, [1.0; 4]);
    o.nine_slice(pic, 24, qx + t * 5.2, qy - t * 1.2, t * 2.6, t * 2.4, t * 0.5, [1.0, 1.0, 1.0, 0.9]);
}
