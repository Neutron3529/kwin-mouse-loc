pub use bindgen::*;
mod bindgen {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/uinput.rs"));
}

use libc::ioctl;
use std::{
    fs::{File, OpenOptions},
    mem,
    os::{fd::AsRawFd /*unix::fs::OpenOptionsExt*/},
    time::Duration,
};

pub struct IoCtl(File, timeval, Vec<u8>);
const SYNC_SLEEP: Duration = Duration::from_millis(1);
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
                bustype: BUS_USB as u16,
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

            // for code in [ABS_X, ABS_Y, ABS_WHEEL] {
            //     ioctl(fd, UI_SET_EVBIT, EV_KEY);
            //     ioctl(fd, UI_SET_ABSBIT, code);
            // }
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
    fn sync(&mut self) {
        use std::io::Write;
        self.event(EV_SYN as u16, SYN_REPORT as u16, 0);
        self.0.write_all(&self.2).unwrap();
        self.0.flush().unwrap();
        self.2.clear();
        self.1.tv_usec = self.1.tv_usec.wrapping_add(1);
        if self.1.tv_usec >= 1_000_000 {
            self.1.tv_sec += 1;
            self.1.tv_usec -= 1_000_000;
        }
        std::thread::sleep(SYNC_SLEEP);
    }
    pub fn send(&mut self, type_: u16, code: u16, val: i32) {
        self.event(type_, code, val);
        self.sync();
    }
    pub fn move_mouse(&mut self, x: i32, y: i32) {
        self.event(EV_REL as u16, REL_X as u16, x);
        self.send(EV_REL as u16, REL_Y as u16, y);
        println!("try move {x} {y}");
    }
    pub fn press(&mut self, btn: u16) {
        self.send(EV_KEY as u16, btn, 1)
    }
    pub fn release(&mut self, btn: u16) {
        self.send(EV_KEY as u16, btn, 0)
    }
}
