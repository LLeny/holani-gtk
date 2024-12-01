mod imp;

use gtk::{gdk, glib, prelude::*, subclass::prelude::*};
use holani::mikey::video::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH};

glib::wrapper! {
    pub struct LynxDisplay(ObjectSubclass<imp::LynxDisplay>) @implements gdk::Paintable;
}

impl Default for LynxDisplay {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl LynxDisplay {

    pub fn setup_next_frame(&self, data: &Vec<u8>) {
        let imp = self.imp();

        let bytes = glib::Bytes::from(data);

        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_bytes(
            &bytes, 
            gtk::gdk_pixbuf::Colorspace::Rgb ,
            false, 
            8, 
            LYNX_SCREEN_WIDTH as i32, 
            LYNX_SCREEN_HEIGHT as i32, 
            LYNX_SCREEN_WIDTH as i32 * 3
        )
        .scale_simple(
            LYNX_SCREEN_WIDTH as i32 * 2, 
            LYNX_SCREEN_HEIGHT as i32 * 2, 
            gtk::gdk_pixbuf::InterpType::Nearest
        ).unwrap();
       
        let texture = gdk::Texture::for_pixbuf(&pixbuf);

        imp.next_frame.replace(Some(texture));

        self.invalidate_contents();
    }
}