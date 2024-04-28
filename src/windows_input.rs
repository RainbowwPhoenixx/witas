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
