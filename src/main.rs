// A macOS keyboard remapper from QWERTY to Dvorak when Command or Control are not pressed.
use clap::{Parser, Subcommand};
use std::error::Error;
use std::os::raw::c_void;
use std::ptr;
use std::{env, fs, process};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

// CoreGraphics types and constants
type CGEventTapLocation = u32;
type CGEventTapPlacement = u32;
type CGEventTapOptions = u32;
type CGEventMask = u64;
type CGEventType = u32;
type CGEventField = u32;
type CGEventFlags = u64;
type CGEventTapProxy = *mut c_void;
type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFStringRef = *const c_void;
type CFAllocatorRef = *const c_void;
// Callback signature
type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    type_: CGEventType,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

// Event types
const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
const K_CG_EVENT_KEY_UP: CGEventType = 11;
// Event fields
const K_CG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
// Tap configuration: event tap location (use HID-level)
const K_CG_HID_EVENT_TAP: CGEventTapLocation = 0;
const K_CG_HEAD_INSERT_EVENT_TAP: CGEventTapPlacement = 0;
const K_CG_EVENT_TAP_OPTION_DEFAULT: CGEventTapOptions = 0;
// Flags to ignore remapping when pressed
const K_CG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 1 << 20;
const K_CG_EVENT_FLAG_MASK_CONTROL: CGEventFlags = 1 << 18;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        eventsOfInterest: CGEventMask,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
}

/// Compute an event mask bit (equivalent to C macro CGEventMaskBit)
fn cg_event_mask_bit(event: CGEventType) -> CGEventMask {
    1u64 << event
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(runLoop: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: CFStringRef;
}

// Command-line interface
#[derive(Parser)]
#[command(
    name = "macos_keyboard_remapper",
    version,
    about = "Remap QWERTY to Dvorak on macOS"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install as a LaunchAgent (auto-start at login)
    Install,
    /// Remove the LaunchAgent
    Uninstall,
}
// Key code constants (from HIToolbox/Events.h, kVK_*):
const VK_A: u64 = 0;
const VK_S: u64 = 1;
const VK_D: u64 = 2;
const VK_F: u64 = 3;
const VK_H: u64 = 4;
const VK_G: u64 = 5;
const VK_Z: u64 = 6;
const VK_X: u64 = 7;
const VK_C: u64 = 8;
const VK_V: u64 = 9;
const VK_B: u64 = 11;
const VK_Q: u64 = 12;
const VK_W: u64 = 13;
const VK_E: u64 = 14;
const VK_R: u64 = 15;
const VK_Y: u64 = 16;
const VK_T: u64 = 17;
const VK_ANSI_1: u64 = 18;
const VK_ANSI_2: u64 = 19;
const VK_ANSI_3: u64 = 20;
const VK_ANSI_4: u64 = 21;
const VK_ANSI_6: u64 = 22;
const VK_ANSI_5: u64 = 23;
const VK_ANSI_EQUALS: u64 = 24;
const VK_ANSI_9: u64 = 25;
const VK_ANSI_7: u64 = 26;
const VK_MINUS: u64 = 27;
const VK_ANSI_8: u64 = 28;
const VK_ANSI_0: u64 = 29;
const VK_RIGHTBRACKET: u64 = 30;
const VK_O: u64 = 31;
const VK_U: u64 = 32;
const VK_LEFTBRACKET: u64 = 33;
const VK_I: u64 = 34;
const VK_P: u64 = 35;
const VK_L: u64 = 37;
const VK_J: u64 = 38;
const VK_QUOTE: u64 = 39;
const VK_K: u64 = 40;
const VK_SEMICOLON: u64 = 41;
const VK_BACKSLASH: u64 = 42;
const VK_COMMA: u64 = 43;
const VK_SLASH: u64 = 44;
const VK_N: u64 = 45;
const VK_M: u64 = 46;
const VK_PERIOD: u64 = 47;

// Text input source detection (to only remap on US QWERTY)
type TISInputSourceRef = *mut c_void;
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> TISInputSourceRef;
    fn TISGetInputSourceProperty(
        source: TISInputSourceRef,
        propertyKey: CFStringRef,
    ) -> CFStringRef;
    static kTISPropertyInputSourceID: CFStringRef;
}
// CoreFoundation helpers
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringGetCStringPtr(theString: CFStringRef, encoding: u32) -> *const i8;
    fn CFStringGetCString(
        theString: CFStringRef,
        buffer: *mut i8,
        bufferSize: isize,
        encoding: u32,
    ) -> bool;
    fn CFRelease(cf: *const c_void);
}
use std::ffi::CStr;
const K_CFSTRING_ENCODING_UTF8: u32 = 0x08000100;

/// Returns true if current keyboard layout is US QWERTY
fn is_us_qwerty() -> bool {
    unsafe {
        let src = TISCopyCurrentKeyboardLayoutInputSource();
        if src.is_null() {
            return false;
        }

        let id_cf = TISGetInputSourceProperty(src, kTISPropertyInputSourceID);
        let mut is_us = false;
        let ptr = CFStringGetCStringPtr(id_cf, K_CFSTRING_ENCODING_UTF8);
        if !ptr.is_null() {
            if let Ok(s) = CStr::from_ptr(ptr).to_str() {
                if s == "com.apple.keyboardlayout.US" || s == "com.apple.keylayout.US" {
                    is_us = true;
                }
            }
        } else {
            let mut buf = [0i8; 256];
            if CFStringGetCString(
                id_cf,
                buf.as_mut_ptr(),
                buf.len() as isize,
                K_CFSTRING_ENCODING_UTF8,
            ) {
                if let Ok(s) = CStr::from_ptr(buf.as_ptr()).to_str() {
                    if s == "com.apple.keyboardlayout.US" || s == "com.apple.keylayout.US" {
                        is_us = true;
                    }
                }
            }
        }

        CFRelease(src as *const c_void);
        is_us
    }
}

// Remap QWERTY keycodes to Dvorak keycodes (only when on US QWERTY layout)
fn remap_key(key: u64) -> Option<u64> {
    if !is_us_qwerty() {
        return None;
    }
    match key {
        // Top row
        VK_Q => Some(VK_QUOTE),                  // Q -> '
        VK_W => Some(VK_COMMA),                  // W -> ,
        VK_E => Some(VK_PERIOD),                 // E -> .
        VK_R => Some(VK_P),                      // R -> P
        VK_T => Some(VK_Y),                      // T -> Y
        VK_Y => Some(VK_F),                      // Y -> F
        VK_U => Some(VK_G),                      // U -> G
        VK_I => Some(VK_C),                      // I -> C
        VK_O => Some(VK_R),                      // O -> R
        VK_P => Some(VK_L),                      // P -> L
        VK_LEFTBRACKET => Some(VK_SLASH),        // [ -> /
        VK_RIGHTBRACKET => Some(VK_ANSI_EQUALS), // ] -> =
        // Home row
        VK_A => Some(VK_A),         // A -> A
        VK_S => Some(VK_O),         // S -> O
        VK_D => Some(VK_E),         // D -> E
        VK_F => Some(VK_U),         // F -> U
        VK_G => Some(VK_I),         // G -> I
        VK_H => Some(VK_D),         // H -> D
        VK_J => Some(VK_H),         // J -> H
        VK_K => Some(VK_T),         // K -> T
        VK_L => Some(VK_N),         // L -> N
        VK_SEMICOLON => Some(VK_S), // ; -> S
        VK_QUOTE => Some(VK_MINUS), // ' -> -
        // Bottom row
        VK_Z => Some(VK_SEMICOLON),         // Z -> ;
        VK_X => Some(VK_Q),                 // X -> Q
        VK_C => Some(VK_J),                 // C -> J
        VK_V => Some(VK_K),                 // V -> K
        VK_B => Some(VK_X),                 // B -> X
        VK_N => Some(VK_B),                 // N -> B
        VK_M => Some(VK_M),                 // M -> M
        VK_COMMA => Some(VK_W),             // , -> W
        VK_PERIOD => Some(VK_V),            // . -> V
        VK_SLASH => Some(VK_Z),             // / -> Z
        VK_BACKSLASH => Some(VK_BACKSLASH), // \ stays \\
        // Number-row punctuation: map '-' and '=' to bracket keys so they produce Dvorak brackets
        VK_MINUS => Some(VK_LEFTBRACKET), // '-' -> '['  (and shift '{')
        VK_ANSI_EQUALS => Some(VK_RIGHTBRACKET), // '=' -> ']'  (and shift '}')
        // Digits (identity mapping)
        VK_ANSI_1 | VK_ANSI_2 | VK_ANSI_3 | VK_ANSI_4 | VK_ANSI_5 | VK_ANSI_6 | VK_ANSI_7
        | VK_ANSI_8 | VK_ANSI_9 | VK_ANSI_0 => Some(key),
        _ => None,
    }
}

// Event tap callback: remap keys when neither Command nor Control is pressed
unsafe extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    type_: CGEventType,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    if type_ == K_CG_EVENT_KEY_DOWN || type_ == K_CG_EVENT_KEY_UP {
        unsafe {
            let flags = CGEventGetFlags(event);
            if flags & (K_CG_EVENT_FLAG_MASK_COMMAND | K_CG_EVENT_FLAG_MASK_CONTROL) == 0 {
                let keycode =
                    CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) as u64;
                if let Some(mapped) = remap_key(keycode) {
                    CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, mapped as i64);
                }
            }
        }
    }
    event
}

fn main() {
    // Initialize tracing subscriber (logs to stderr by default)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .init();
    // Parse command-line arguments
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Install) => {
            if let Err(e) = do_install() {
                error!(%e, "Install failed");
                process::exit(1);
            }
        }
        Some(Commands::Uninstall) => {
            if let Err(e) = do_uninstall() {
                error!(%e, "Uninstall failed");
                process::exit(1);
            }
        }
        None => {
            run_tap();
        }
    }
}

// Install as a LaunchAgent
fn do_install() -> Result<(), Box<dyn Error>> {
    let exe = env::current_exe()?;
    let home = env::var("HOME")?;
    let la_dir = format!("{}/Library/LaunchAgents", home);
    fs::create_dir_all(&la_dir)?;
    let plist_path = format!("{}/com.macos_keyboard_remapper.plist", la_dir);
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.macos_keyboard_remapper</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/macos_keyboard_remapper.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/macos_keyboard_remapper.log</string>
</dict>
</plist>
"#,
        exe.display()
    );
    // Write the LaunchAgent plist
    fs::write(&plist_path, plist)?;
    info!("Created LaunchAgent plist at {}", plist_path);
    // Reload agent
    // Unload any existing agent
    info!("Unloading existing LaunchAgent (if any)");
    let _ = process::Command::new("launchctl")
        .args(&["unload", &plist_path])
        .output();
    // Load the new agent
    info!("Loading LaunchAgent");
    let _ = process::Command::new("launchctl")
        .args(&["load", &plist_path])
        .output()?;
    info!("LaunchAgent installed and loaded");
    Ok(())
}

// Uninstall LaunchAgent
fn do_uninstall() -> Result<(), Box<dyn Error>> {
    let home = env::var("HOME")?;
    let plist_path = format!(
        "{}/Library/LaunchAgents/com.macos_keyboard_remapper.plist",
        home
    );
    // Unload the LaunchAgent
    info!("Unloading LaunchAgent");
    let _ = process::Command::new("launchctl")
        .args(&["unload", &plist_path])
        .output();
    // Remove the plist file
    fs::remove_file(&plist_path)?;
    info!("LaunchAgent removed (plist deleted)");
    Ok(())
}

// Run the keyboard remapping event tap (never returns)
fn run_tap() -> ! {
    unsafe {
        let mask = cg_event_mask_bit(K_CG_EVENT_KEY_DOWN) | cg_event_mask_bit(K_CG_EVENT_KEY_UP);
        let tap = CGEventTapCreate(
            K_CG_HID_EVENT_TAP,
            K_CG_HEAD_INSERT_EVENT_TAP,
            K_CG_EVENT_TAP_OPTION_DEFAULT,
            mask,
            event_tap_callback,
            ptr::null_mut(),
        );
        if tap.is_null() {
            error!("Failed to create event tap. Make sure to grant accessibility permissions.");
            process::exit(1);
        }
        let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        CFRunLoopRun();
        process::exit(0);
    }
}
