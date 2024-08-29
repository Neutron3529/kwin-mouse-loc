# kwin-mouse-loc

A very simple mouse controller that uses `libc::process_vm_readv` to read mouse location. Need root permissions.

# Usage

Since mouse and keyboard operations is very dangerous, it might be easily be poisoned.
And since there is no guarateen that crate owner is not evil, I wrote this simple crate.

The main aim of this crate is that, make user ensure they use a *SAFE* crate that cannot be poisoned.
The *BEST* practice of using this crate should be just copy the `build.rs` and `lib.rs` into your project. 

If you like this crate, you could make it as an optional dependencies, BUT please keep one thing in mind:

> DO NOT ENABLE THE DEPENDENCIES OF THIS CRATE

# Example

```rust
use kwin_mouse_loc::pointer::Workspace;
fn main(){
    let mouse = unsafe{Workspace::new().get_mouse()};
    let (x,y) = mouse.loc();
    println!("mouse is located at ({x}, {y})");
    // do some other things.
    std::thread::sleep(std::time::Duration::from_millis(300));

    // obtain mouse location again, with display.
    println!("mouse is located at {mouse}");
}
```
