#![allow(unused, non_camel_case_types, non_snake_case)]

// Thank you stack overflow
macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<u32> for $name {
            type Error = ();

            fn try_from(v: u32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u32 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Default)]
    pub enum Message {
        WM_NCMOUSEMOVE = 160,
        #[default]
        WM_INPUT = 255,
        WM_KEYDOWN = 256,
        WM_KEYUP = 257,
        WM_CHAR = 258,
        WM_DEADCHAR = 259,
        WM_SYSKEYDOWN = 260,
        WM_SYSKEYUP = 261,
        WM_SYSCOMMAND = 274,
        WM_MOUSEMOVE = 512,
        WM_LBUTTONDOWN = 513,
        WM_LBUTTONUP = 514,
        WM_LBUTTONDLBCLK = 515,
        WM_RBUTTONDOWN = 516,
        WM_RBUTTONUP = 517,
        WM_RBUTTONDBCLK = 518,
        WM_MBUTTONDOWN = 519,
        WM_MBUTTONUP = 520,
        WM_XBUTTONDOWN = 523,
        WM_XBUTTONUP = 524,
        WM_POINTERUPDATE = 581,
        WM_POINTERDOWN = 582,
        WM_POINTERUP = 583,
        WM_POINTERCAPTURECHANGED = 588,
    }
}

#[derive(Default)]
pub struct POINT {
    x: u64,
    y: u64,
}

#[derive(Default)]
pub struct MSG {
    hwnd: usize,
    message: Message,
    wParam: usize,
    lParam: isize,
    time: u32,
    pt: POINT,
    lPrivate: u32,
}

// Some useful defs
back_to_enum! {
    #[repr(C)]
    pub enum VirtualKeyCode {
        // Mouse
        LButton = 0x01,
        RButton = 0x02,

        Space = 0x20,

        // Numbers
        NUM_0 = 0x30,
        NUM_1 = 0x31,
        NUM_2 = 0x32,
        NUM_3 = 0x33,
        NUM_4 = 0x34,
        NUM_5 = 0x35,
        NUM_6 = 0x36,
        NUM_7 = 0x37,
        NUM_8 = 0x38,
        NUM_9 = 0x39,

        // Movement keys
        A = 0x41,
        B = 0x42,
        C = 0x43,
        D = 0x44,
        E = 0x45,
        F = 0x46,
        G = 0x47,
        H = 0x48,
        I = 0x49,
        J = 0x4A,
        K = 0x4B,
        L = 0x4C,
        M = 0x4D,
        N = 0x4E,
        O = 0x4F,
        P = 0x50,
        Q = 0x51,
        R = 0x52,
        S = 0x53,
        T = 0x54,
        U = 0x55,
        V = 0x56,
        W = 0x57,
        X = 0x58,
        Y = 0x59,
        Z = 0x5A,


        LShift = 0xA0,
    }
}
