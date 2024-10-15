pub use bindgen::*;
mod bindgen {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/uinput.rs"));
    pub fn parse(x: impl ParseKeyCode) -> u32 {
        x.parse_keycode()
    }
    pub trait ParseKeyCode {
        fn parse_keycode(self) -> u32;
    }
    impl<'a> ParseKeyCode for &'a str {
        fn parse_keycode(self) -> u32 {
            match self {
                "a" => KEY_A,
                "b" => KEY_B,
                "c" => KEY_C,
                "d" => KEY_D,
                "e" => KEY_E,
                "f" => KEY_F,
                "g" => KEY_G,
                "h" => KEY_H,
                "i" => KEY_I,
                "j" => KEY_J,
                "k" => KEY_K,
                "l" => KEY_L,
                "m" => KEY_M,
                "n" => KEY_N,
                "o" => KEY_O,
                "p" => KEY_P,
                "q" => KEY_Q,
                "r" => KEY_R,
                "s" => KEY_S,
                "t" => KEY_T,
                "u" => KEY_U,
                "v" => KEY_V,
                "w" => KEY_W,
                "x" => KEY_X,
                "y" => KEY_Y,
                "z" => KEY_Z,
                "shift" | "lshift" => KEY_LEFTSHIFT,
                "rshift" => KEY_RIGHTSHIFT,
                "esc" => KEY_ESC,
                "W" | "up" => KEY_UP,
                "S" | "down" => KEY_DOWN,
                "A" | "left" => KEY_LEFT,
                "D" | "right" => KEY_RIGHT,
                "L" | "click" => BTN_LEFT,
                "R" | "rclick" => BTN_RIGHT,
                "M" | "middle" => BTN_MIDDLE,
                " " | "space" => KEY_SPACE,
                "\n" | "enter" => KEY_ENTER,
                _ => {
                    eprintln! {"Unsupported {self}, converted to BTN_LEFT. Use uppercase WSAD for key up/down/left/right, LMR for mouse left/middle/right key. Uppercase are often not what it looks like, be careful."}
                    BTN_LEFT
                }
            }
        }
    }
    impl ParseKeyCode for char {
        fn parse_keycode(self) -> u32 {
            match self {
                'W' => KEY_UP,
                'S' => KEY_DOWN,
                'A' => KEY_LEFT,
                'D' => KEY_RIGHT,
                'L' => BTN_LEFT,
                'R' => BTN_RIGHT,
                'M' => BTN_MIDDLE,
                'a' => KEY_A,
                'B' | 'b' => KEY_B,
                'C' | 'c' => KEY_C,
                'd' => KEY_D,
                'E' | 'e' => KEY_E,
                'F' | 'f' => KEY_F,
                'G' | 'g' => KEY_G,
                'H' | 'h' => KEY_H,
                'I' | 'i' => KEY_I,
                'J' | 'j' => KEY_J,
                'K' | 'k' => KEY_K,
                'l' => KEY_L,
                'm' => KEY_M,
                'N' | 'n' => KEY_N,
                'O' | 'o' => KEY_O,
                'P' | 'p' => KEY_P,
                'Q' | 'q' => KEY_Q,
                'r' => KEY_R,
                's' => KEY_S,
                'T' | 't' => KEY_T,
                'U' | 'u' => KEY_U,
                'V' | 'v' => KEY_V,
                'w' => KEY_W,
                'X' | 'x' => KEY_X,
                'Y' | 'y' => KEY_Y,
                'Z' | 'z' => KEY_Z,
                ' ' => KEY_SPACE,
                '\n' => KEY_ENTER,
                '\t' => KEY_TAB,
                ';' => KEY_SEMICOLON,
                '\'' => KEY_APOSTROPHE,
                '[' => KEY_LEFTBRACE,
                ']' => KEY_RIGHTBRACE,
                '\\' => KEY_BACKSLASH,
                '/' => KEY_SLASH,
                ',' => KEY_COMMA,
                '.' => KEY_DOT,

                '1' => KEY_1,
                '2' => KEY_2,
                '3' => KEY_3,
                '4' => KEY_4,
                '5' => KEY_5,
                '6' => KEY_6,
                '7' => KEY_7,
                '8' => KEY_8,
                '9' => KEY_9,
                '0' => KEY_0,
                '-' => KEY_MINUS,
                '=' => KEY_EQUAL,

                _ => {
                    eprintln! {"Unsupported {self}, converted to KEY_ESC. Use uppercase WSAD for key up/down/left/right, LMR for mouse left/middle/right key. Uppercase are often not what it looks like, be careful."}
                    KEY_ESC
                }
            }
        }
    }
}

use libc::ioctl;
use std::{
    fs::{File, OpenOptions},
    mem,
    os::{fd::AsRawFd /*unix::fs::OpenOptionsExt*/},
    time::Duration,
};

pub struct IoCtl(File, timeval, Vec<u8>);
impl Drop for IoCtl {
    fn drop(&mut self) {
        unsafe {
            ioctl(self.0.as_raw_fd(), UI_DEV_DESTROY);
        }
    }
}
impl IoCtl {
    pub fn new() -> Self {
        let file = OpenOptions::new()
            .write(true)
            // .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")
            .unwrap();
        let mut name = [0; 80];
        name.iter_mut()
            .zip("my-virtual-mouse".bytes().map(|x| x as i8))
            .for_each(|(n, s)| *n = s);
        let definition = uinput_setup {
            id: input_id {
                bustype: BUS_VIRTUAL as u16,
                vendor: 0x045e,
                product: 0x07a5,
                version: 0x0111,
            },
            name,
            ff_effects_max: 0,
        };
        let fd = file.as_raw_fd();
        unsafe {
            for code in [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE] {
                ioctl(fd, UI_SET_EVBIT, EV_KEY);
                ioctl(fd, UI_SET_KEYBIT, code);
            }

            ioctl(fd, UI_SET_EVBIT, EV_REL);
            ioctl(fd, UI_SET_RELBIT, REL_X);
            ioctl(fd, UI_SET_RELBIT, REL_Y);
            // no need to register ABSBIT since it is touchpad related things.
            #[cfg(feature = "keyboard")]
            for code in KEY_RESERVED..=KEY_MICMUTE {
                ioctl(fd, UI_SET_EVBIT, EV_KEY);
                ioctl(fd, UI_SET_KEYBIT, code);
            }

            ioctl(fd, UI_DEV_SETUP, &definition);
            ioctl(fd, UI_DEV_CREATE);
        }
        Self(
            file,
            timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            Vec::with_capacity(100),
        )
    }
    fn event(&mut self, type_: u16, code: u16, value: i32) {
        use std::io::Write;
        self.2
            .write_all(unsafe {
                &*(&input_event {
                    time: self.1,
                    type_,
                    code,
                    value,
                } as *const input_event
                    as *const [u8; mem::size_of::<input_event>()])
            })
            .unwrap();
    }
    pub fn send(&mut self, type_: u16, code: u16, val: i32) {
        self.event(type_, code, val);
        self.sync();
    }
    pub fn move_mouse(&mut self, x: i32, y: i32) {
        self.event(EV_REL as u16, REL_X as u16, x);
        self.send(EV_REL as u16, REL_Y as u16, y);
    }
    pub fn press(&mut self, btn: impl IntoU16) {
        self.send(EV_KEY as u16, btn.into(), 1)
    }
    pub fn release(&mut self, btn: impl IntoU16) {
        self.send(EV_KEY as u16, btn.into(), 0)
    }
    pub fn click(&mut self, btn: impl IntoU16, half_dur: Duration) {
        self.press(btn.into());
        std::thread::sleep(half_dur);
        self.release(btn.into());
        std::thread::sleep(half_dur);
    }
    fn sync(&mut self) {
        use std::io::Write;
        self.event(EV_SYN as u16, SYN_REPORT as u16, 0);
        self.0.write_all(&self.2).unwrap();
        self.0.flush().unwrap();
        self.2.clear();
        self.1.tv_usec = self.1.tv_usec.wrapping_add(1_000);
        if self.1.tv_usec >= 1_000_000 {
            self.1.tv_sec += 1;
            self.1.tv_usec -= 1_000_000;
        }
    }
}
pub trait IntoU16: Copy {
    fn into(self) -> u16;
}
impl IntoU16 for u32 {
    #[inline(always)]
    fn into(self) -> u16 {
        self as u16
    }
}
impl IntoU16 for u16 {
    #[inline(always)]
    fn into(self) -> u16 {
        self
    }
}
