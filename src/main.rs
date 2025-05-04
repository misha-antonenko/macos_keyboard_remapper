// A macOS keyboard remapper from QWERTY to Dvorak when Command or Control are not pressed.
use std::os::raw::c_void;
use std::ptr;

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
extern "C" {
    fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        eventsOfInterest: CGEventMask,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    // fn CGEventMaskBit(event: CGEventType) -> CGEventMask; // C macro, not an actual symbol
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: CGEventField, value: i64);
}

/// Compute an event mask bit (equivalent to C macro CGEventMaskBit)
fn cg_event_mask_bit(event: CGEventType) -> CGEventMask {
    1u64 << event
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(
        runLoop: CFRunLoopRef,
        source: CFRunLoopSourceRef,
        mode: CFStringRef,
    );
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: CFStringRef;
}

// Remap QWERTY keycodes to Dvorak keycodes
fn remap_key(key: u64) -> Option<u64> {
    match key {
        12 => Some(39), // Q -> '
        13 => Some(43), // W -> ,
        14 => Some(47), // E -> .
        15 => Some(35), // R -> P
        17 => Some(16), // T -> Y
        16 => Some(3),  // Y -> F
        32 => Some(5),  // U -> G
        34 => Some(8),  // I -> C
        31 => Some(15), // O -> R
        35 => Some(37), // P -> L
        33 => Some(44), // [ -> /
        30 => Some(24), // ] -> =
        0 => Some(0),   // A -> A
        1 => Some(31),  // S -> O
        2 => Some(14),  // D -> E
        3 => Some(32),  // F -> U
        5 => Some(34),  // G -> I
        4 => Some(2),   // H -> D
        38 => Some(4),  // J -> H
        40 => Some(17), // K -> T
        37 => Some(45), // L -> N
        41 => Some(1),  // ; -> S
        39 => Some(27), // ' -> -
        6 => Some(41),  // Z -> ;
        7 => Some(12),  // X -> Q
        8 => Some(38),  // C -> J
        9 => Some(40),  // V -> K
        11 => Some(7),  // B -> X
        45 => Some(11), // N -> B
        46 => Some(46), // M -> M
        43 => Some(13), // , -> W
        47 => Some(9),  // . -> V
        44 => Some(6),  // / -> Z
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
        let flags = CGEventGetFlags(event);
        if flags & (K_CG_EVENT_FLAG_MASK_COMMAND | K_CG_EVENT_FLAG_MASK_CONTROL) == 0 {
            let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) as u64;
            if let Some(mapped) = remap_key(keycode) {
                println!("remapping {} to {}", keycode, mapped);
                CGEventSetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE, mapped as i64);
            }
        }
    }
    event
}

fn main() {
    unsafe {
        // Listen for key down and key up events
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
            eprintln!("Failed to create event tap. Make sure to grant accessibility permissions.");
            std::process::exit(1);
        }
        let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        // Enable and run
        CGEventTapEnable(tap, true);
        CFRunLoopRun();
    }
}
