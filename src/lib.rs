#![warn(unsafe_op_in_unsafe_fn)]
mod consts;
/// pointer of kwin workspace and its cursor's position
pub mod pointer {
    use crate::consts::*;
    use libc::{iovec, process_vm_readv};
    use std::{ffi::c_void, fmt::Display, fs::File, io::Read, process::Command, ptr};
    /// PID of kwin_wayland.
    /// SAFETY: users should ensure this is the pid of kwin_wayland, and this PID is valid before this program exited.
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct KWinPid(i32);
    impl KWinPid {
        /// SAFETY: users should ensure this is the pid of kwin_wayland, and this PID is valid before this program exited.
        pub unsafe fn from(i: i32) -> Self {
            Self(i)
        }
        /// SAFETY: users should ensure this is the pid of kwin_wayland, and this PID is valid before this program exited.
        pub unsafe fn search(all_user: bool) -> Self {
            Self(
                String::from_utf8_lossy(
                    &Command::new("ps")
                        .arg(if all_user { "ax" } else { "x" }) // "a" is needed since there might not be a wayland window running by root.
                        .output()
                        .expect("cannot enumerate programs")
                        .stdout,
                )
                .lines()
                .filter(|x| x.contains("/kwin_wayland "))
                .next()
                .expect("failed to find kwin_wayland session")
                .trim()
                .split_once(' ')
                .expect("cannot parse `ps`'s output")
                .0
                .parse()
                .expect("cannot parse the pid"),
            )
        }
    }
    #[derive(Eq, PartialEq)]
    /// pointer of workspace
    pub struct Workspace(KWinPid, *mut c_void);
    impl Workspace {
        /// Automatically create a workspace pointer with default offset (might be wrong!) and automatically detected kwin_wayland (may also wrong!).
        /// Use for test and demo only.
        /// SAFETY: Ensure the WORKSPACE_OFFSET is correct.
        ///
        /// require root permissions to calculate the workspace's offset.
        pub unsafe fn new(search_all_user: bool) -> Self {
            unsafe { Self::get(KWinPid::search(search_all_user), WORKSPACE_OFFSET) }
        }
        /// get workspace from kwin_wayland, the pid should met kwin_wayland's pid, otherwise I cannot tell what happens.
        /// since it relys on reading "/proc/{pid}/maps", root access might be needed.
        ///
        /// It will use an offset that calculated in compile-time, if runtime detect is needed, using `get_with_readelf` instead (executable`readelf` should in enviroment `$PATH`)
        ///
        /// require root permissions to calculate the workspace's offset.
        pub fn get(pid: KWinPid, workspace_offset: usize) -> Self {
            let mut buffer = String::new();

            // require root permissions
            File::open(&format!("/proc/{}/maps", pid.0))
                .unwrap_or_else(|e| panic!("cannot open file (require permissions?)\n{:?}", e))
                .read_to_string(&mut buffer)
                .expect("read maps failed");
            let buffer0 = buffer
                .split_once("libkwin.so")
                .expect("program does not load libkwin.so (is it really kwin_wayland?)")
                .0;
            let buffer1 = buffer0.rsplit_once('\n').unwrap_or(("", buffer0)).1.trim();
            // 70642a400000-70642a54a000 r--p 00000000 103:02 3323906                   /usr/lib/libkwin.so.6.1.4
            let Some((offset, start)) = buffer1.split_once(" r--p ") else {
                panic!("get offset failed, the buffer line is `{buffer1}`")
            };
            assert!(start.trim().starts_with("00000000"));
            let offset1 = offset.split_once('-').expect("maps format error").0;
            let base =
                usize::from_str_radix(offset1, 16).expect("cannot parse to base 16") as *mut c_void;
            let ret = unsafe { base.byte_add(workspace_offset) };
            println!("base offset: {base:?}, {ret:?}");
            Self(pid, ret)
        }
        /// using `readelf` to detect the true offset in `path_to_libkwin.so`.
        ///
        /// By default, param `readelf` could be str `"readelf"` since the executable `readelf` often in $PATH.
        /// And set `path_to_libkwin` to `"/usr/lib/libkwin.so"` suits most of the cases.
        pub fn get_offset_with_readelf(readelf: &str, path_to_libkwin: &str) -> usize {
            usize::from_str_radix(
                &String::from_utf8(
                    Command::new(readelf)
                        .args(["-WCs", path_to_libkwin])
                        .output()
                        .expect("readelf execute failed")
                        .stdout,
                )
                .expect("failed to parse readelf")
                .split_once(r#"KWin::Workspace::_self"#)
                .expect("cannot find KWin::Workspace::_self")
                .0
                .rsplit_once('\n')
                .expect("cannot read offset of KWin::Workspace::_self")
                .1
                .split_once(':')
                .expect("parse `:` failed.")
                .1
                .trim()
                .split_once(' ')
                .expect("cannot parse space")
                .0,
                16,
            )
            .expect("failed to process readelf.")
        }

        /// get mouse_pos offset from pointer of workspace.
        ///
        /// Due to unsafety of KWinPid, this function is actually unsafe. Caller should ensure the pid is still valid.
        pub fn get_mouse(&self) -> Mouse {
            let mut addr: *mut c_void = ptr::null_mut();
            let local = iovec {
                iov_base: &mut addr as *mut _ as *mut c_void,
                iov_len: 8,
            };
            let remote = iovec {
                iov_base: self.1,
                iov_len: 8,
            };
            // SAFETY: As KWinPid suggests, the safety of KWinPid ensure that the pid is valid,
            //         Since offset is ensured to be valid, the result is safe.
            match unsafe { process_vm_readv(self.0 .0, &local, 1, &remote, 1, 0) } {
                8 => assert!(!addr.is_null()),
                -1 => {
                    eprintln!("failed, check errno for more details.")
                }
                x => eprintln!("unknown bytes readed: {x}"),
            }
            // SAFETY: the offset is readed by bindgen.
            Mouse(self.0, unsafe { addr.byte_add(POS_OFFSET) })
        }
    }
    /// pointer of focusMousePos
    #[derive(Eq, PartialEq)]
    pub struct Mouse(KWinPid, *mut c_void);
    impl Mouse {
        /// read mouse location from kwin workspace (it is read-only object, cannot write back.)
        pub fn loc(&self) -> (f64, f64) {
            let mut xy = [0f64; 2];
            let local = iovec {
                iov_base: xy.as_mut_ptr() as *mut c_void,
                iov_len: 16,
            };
            let remote = iovec {
                iov_base: self.1,
                iov_len: 16,
            };
            // SAFETY: If you could read the code, it is safe.
            //         Otherwise it is very unsafe.
            match unsafe { process_vm_readv(self.0 .0, &local, 1, &remote, 1, 0) } {
                16 => return (xy[0], xy[1]),
                -1 => {
                    eprintln!("failed, check errno for more details.")
                }
                x => eprintln!("unknown bytes readed: {x}"),
            }
            panic!("reading failed.");
        }
    }
    /// allow print mouse location directly.
    impl Display for Mouse {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(&self.loc(), f)
        }
    }
}
#[cfg(test)]
mod test {
    use crate::*;
    use consts::WORKSPACE_OFFSET;
    use pointer::{KWinPid, Workspace};
    #[test]
    fn equality() {
        let w1 = unsafe { Workspace::new(true) }; // most simple way

        let pid = unsafe { KWinPid::search(true) }; // calc pid
        let offset = Workspace::get_offset_with_readelf("readelf", "/usr/lib/libkwin.so"); // calc offset
        let w2 = Workspace::get(pid, offset); // get workspace from pid and offset
        assert!(w1 == w2);
        assert!(WORKSPACE_OFFSET == offset);
    }
    #[test]
    fn get_loc() {
        let workspace = unsafe { Workspace::new(true) };
        let mouse = workspace.get_mouse();
        println!("{:?} {}", mouse.loc(), mouse);
    }
}
