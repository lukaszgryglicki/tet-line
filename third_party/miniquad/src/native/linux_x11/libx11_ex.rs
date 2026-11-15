// little helpers for LibX11
#![allow(non_snake_case)]

use super::*;

/// A monitor as Xrandr 1.5 describes it.
#[repr(C)]
pub struct XRRMonitorInfo {
    pub name: Atom,
    pub primary: libc::c_int,
    pub automatic: libc::c_int,
    pub noutput: libc::c_int,
    pub x: libc::c_int,
    pub y: libc::c_int,
    pub width: libc::c_int,
    pub height: libc::c_int,
    pub mwidth: libc::c_int,
    pub mheight: libc::c_int,
    pub outputs: *mut libc::c_ulong,
}

crate::declare_module!(
    LibXrandr,
    "libXrandr.so",
    "libXrandr.so.2",
    ...
    ...
    pub fn XRRGetMonitors(*mut Display, Window, libc::c_int, *mut libc::c_int) -> *mut XRRMonitorInfo,
    pub fn XRRFreeMonitors(*mut XRRMonitorInfo),
    ...
    ...
);

impl LibX11 {
    pub unsafe fn update_system_dpi(&mut self, display: *mut Display) -> f32 {
        let mut dpi_scale = 1.;
        let rms = (self.XResourceManagerString)(display);
        if !rms.is_null() {
            let db = (self.XrmGetStringDatabase)(rms);
            if !db.is_null() {
                let mut value = XrmValue {
                    size: 0,
                    addr: std::ptr::null_mut::<libc::c_char>(),
                };
                let mut type_ = std::ptr::null_mut();
                if (self.XrmGetResource)(
                    db,
                    b"Xft.dpi\x00".as_ptr() as _,
                    b"Xft.Dpi\x00".as_ptr() as _,
                    &mut type_,
                    &mut value,
                ) != 0
                {
                    if !type_.is_null() && libc::strcmp(type_, b"String\x00".as_ptr() as _) == 0 {
                        dpi_scale = libc::atof(value.addr as *const _) as f32 / 96.0;
                    }
                }
                (self.XrmDestroyDatabase)(db);
            }
        };
        dpi_scale
    }

    pub unsafe fn grab_error_handler(&mut self) {
        pub unsafe extern "C" fn _sapp_x11_error_handler(
            mut _display: *mut Display,
            event: *mut XErrorEvent,
        ) -> libc::c_int {
            eprintln!("Error: {}", (*event).error_code);
            0 as libc::c_int
        }

        (self.XSetErrorHandler)(Some(
            _sapp_x11_error_handler
                as unsafe extern "C" fn(_: *mut Display, _: *mut XErrorEvent) -> libc::c_int,
        ));
    }

    pub unsafe fn release_error_handler(&mut self, display: *mut Display) {
        (self.XSync)(display, false as _);
        (self.XSetErrorHandler)(None);
    }

    pub unsafe fn query_window_size(
        &mut self,
        display: *mut Display,
        window: Window,
    ) -> (i32, i32) {
        let mut attribs: XWindowAttributes = std::mem::zeroed();
        (self.XGetWindowAttributes)(display, window, &mut attribs);
        (attribs.width, attribs.height)
    }

    pub unsafe fn update_window_title(
        &mut self,
        display: *mut Display,
        window: Window,
        title: &str,
    ) {
        let c_title = std::ffi::CString::new(title).unwrap();

        (self.Xutf8SetWMProperties)(
            display,
            window,
            c_title.as_ptr(),
            c_title.as_ptr(),
            std::ptr::null_mut(),
            0 as libc::c_int,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        (self.XChangeProperty)(
            display,
            window,
            self.extensions.net_wm_name,
            self.extensions.utf8_string,
            8,
            PropModeReplace,
            c_title.as_ptr() as *mut libc::c_uchar,
            c_title.as_bytes().len() as libc::c_int,
        );
        (self.XChangeProperty)(
            display,
            window,
            self.extensions.net_wm_icon_name,
            self.extensions.utf8_string,
            8 as libc::c_int,
            PropModeReplace,
            c_title.as_ptr() as *mut libc::c_uchar,
            c_title.as_bytes().len() as libc::c_int,
        );
        (self.XFlush)(display);
    }

    pub unsafe fn update_window_icon(
        &mut self,
        display: *mut Display,
        window: Window,
        icon: &crate::conf::Icon,
    ) {
        let bytes_length = (icon.small.len() + icon.medium.len() + icon.big.len()) / 4 + 3 * 2;

        // "This is an array of 32bit packed CARDINAL ARGB" (c) freedesktop
        // While, in fact, X11 expect this to be 64bit on 64bit systems, which is proved
        // by quite a few "unsigned long icon[]" I've seen in quite a few real-life programs.
        // Leaving this comment nn case of icon-related bugs in the future.
        let mut icon_bytes: Vec<usize> = vec![0; bytes_length];
        let icons = [
            (16, &icon.small[..]),
            (32, &icon.medium[..]),
            (64, &icon.big[..]),
        ];

        {
            let mut target = icon_bytes.iter_mut();
            for (dim, pixels) in icons {
                *target.next().unwrap() = dim;
                *target.next().unwrap() = dim;
                for pixels in pixels.chunks(4) {
                    let r = pixels[0] as u32;
                    let g = pixels[1] as u32;
                    let b = pixels[2] as u32;
                    let a = pixels[3] as u32;

                    *target.next().unwrap() =
                        ((r << 16) | (g << 8) | (b << 0) | (a << 24)) as usize;
                }
            }
        }

        (self.XChangeProperty)(
            display,
            window,
            self.extensions.net_wm_icon,
            self.extensions.cardinal,
            32,
            PropModeReplace,
            icon_bytes.as_mut_ptr() as *mut _ as *mut _,
            icon_bytes.len() as _,
        );
    }

    pub unsafe fn create_window(
        &mut self,
        root: Window,
        display: *mut Display,
        visual: *mut Visual,
        depth: libc::c_int,
        conf: &crate::conf::Conf,
    ) -> Window {
        let mut wa = XSetWindowAttributes {
            background_pixmap: 0,
            background_pixel: 0,
            border_pixmap: 0,
            border_pixel: 0,
            bit_gravity: 0,
            win_gravity: 0,
            backing_store: 0,
            backing_planes: 0,
            backing_pixel: 0,
            save_under: 0,
            event_mask: 0,
            do_not_propagate_mask: 0,
            override_redirect: 0,
            colormap: 0,
            cursor: 0,
        };
        libc::memset(
            &mut wa as *mut XSetWindowAttributes as *mut libc::c_void,
            0 as libc::c_int,
            ::std::mem::size_of::<XSetWindowAttributes>() as _,
        );
        let wamask = (CWBorderPixel | CWColormap | CWEventMask) as u32;

        if !visual.is_null() {
            let colormap = (self.XCreateColormap)(display, root, visual, AllocNone);
            wa.colormap = colormap;
        }
        wa.border_pixel = 0 as libc::c_int as libc::c_ulong;
        wa.event_mask = StructureNotifyMask
            | KeyPressMask
            | KeyReleaseMask
            | PointerMotionMask
            | ButtonPressMask
            | ButtonReleaseMask
            | ExposureMask
            | FocusChangeMask
            | VisibilityChangeMask
            | EnterWindowMask
            | LeaveWindowMask
            | PropertyChangeMask;
        self.grab_error_handler();

        let (mut width, mut height) = (conf.window_width, conf.window_height);
        if conf.platform.linux_x11_hidden && conf.fullscreen {
            // No window manager makes a window that is never shown full screen: it gets the
            // size of the primary monitor, or of the screen where Xrandr does not tell.
            let private = display as _XPrivDisplay;
            let screen = (*private).screens.offset((*private).default_screen as isize);
            width = (*screen).width;
            height = (*screen).height;
            if let Ok(xrandr) = LibXrandr::try_load() {
                let mut count = 0;
                let monitors = (xrandr.XRRGetMonitors)(display, root, 1, &mut count);
                if !monitors.is_null() {
                    let all = std::slice::from_raw_parts(monitors, count.max(0) as usize);
                    let primary = all.iter().find(|monitor| monitor.primary != 0);
                    if let Some(monitor) = primary.or(all.first()) {
                        width = monitor.width;
                        height = monitor.height;
                    }
                    (xrandr.XRRFreeMonitors)(monitors);
                }
                // Xlib calls the extension when the display is closed: it stays loaded.
                std::mem::forget(xrandr);
            }
        }
        let window = (self.XCreateWindow)(
            display,
            root,
            0 as libc::c_int,
            0 as libc::c_int,
            width as _,
            height as _,
            0 as libc::c_int as libc::c_uint,
            depth,
            InputOutput as libc::c_uint,
            visual,
            wamask as libc::c_ulong,
            &mut wa,
        );
        self.release_error_handler(display);
        assert!(window != 0, "X11: Failed to create window");

        let mut protocols: [Atom; 1] = [self.extensions.wm_delete_window];
        (self.XSetWMProtocols)(display, window, protocols.as_mut_ptr(), 1 as libc::c_int);
        let hints = (self.XAllocSizeHints)();
        (*hints).flags |= PWinGravity;
        if !conf.window_resizable {
            (*hints).flags |= PMinSize | PMaxSize;
            (*hints).min_width = conf.window_width;
            (*hints).min_height = conf.window_height;
            (*hints).max_width = conf.window_width;
            (*hints).max_height = conf.window_height;
        }
        (*hints).win_gravity = StaticGravity;
        (self.XSetWMNormalHints)(display, window, hints);
        (self.XFree)(hints as *mut libc::c_void);

        let class_hint = (self.XAllocClassHint)();
        let wm_class = std::ffi::CString::new(conf.platform.linux_wm_class).unwrap();
        (*class_hint).res_name = wm_class.as_ptr() as _;
        (*class_hint).res_class = wm_class.as_ptr() as _;
        (self.XSetClassHint)(display, window, class_hint);
        (self.XFree)(class_hint as *mut libc::c_void);

        if let Some(ref icon) = conf.icon {
            self.update_window_icon(display, window, icon);
        }
        // For an unknown reason, this should happen after update_window_icon,
        // otherwise X11 will skip ChangeProperty(WM_ICON)
        self.update_window_title(display, window, &conf.window_title);

        window
    }

    pub unsafe fn show_window(&mut self, display: *mut Display, window: Window) {
        (self.XMapWindow)(display, window);
        (self.XRaiseWindow)(display, window);
        (self.XFlush)(display);
    }
}
