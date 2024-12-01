use std::cell::RefCell;

use gtk::{gdk, glib, graphene, prelude::*, subclass::prelude::*};

#[derive(Default)]
pub struct LynxDisplay {
    pub next_frame: RefCell<Option<gdk::Texture>>,
}

#[glib::object_subclass]
impl ObjectSubclass for LynxDisplay {
    const NAME: &'static str = "LynxDisplay";
    type Type = super::LynxDisplay;
    type Interfaces = (gdk::Paintable,);
}

impl ObjectImpl for LynxDisplay {}

impl PaintableImpl for LynxDisplay {
    fn intrinsic_height(&self) -> i32 {
        self.next_frame
            .borrow()
            .as_ref()
            .map(|texture| texture.height())
            .unwrap_or(-1)
    }

    fn intrinsic_width(&self) -> i32 {
        self.next_frame
            .borrow()
            .as_ref()
            .map(|texture| texture.width())
            .unwrap_or(-1)
    }

    fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
        if let Some(texture) = &*self.next_frame.borrow() {
            texture.snapshot(snapshot, width, height);
        } else {
            snapshot.append_color(
                &gdk::RGBA::BLACK,
                &graphene::Rect::new(0f32, 0f32, width as f32, height as f32),
            );
        }
    }
}