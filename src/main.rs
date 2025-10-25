mod wayland;

use crate::wayland::wlr::{
    layer_shell::{zwlr_layer_shell_v1, zwlr_layer_surface_v1},
    output_management::{self, zwlr_output_head_v1, zwlr_output_manager_v1},
};
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_display, wl_output, wl_region, wl_registry, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

use anyhow::Result;
use image::DynamicImage;
use memmap2::MmapMut;
use std::{env, io::Write, os::fd::AsFd, path::PathBuf};
use tempfile::tempfile;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
};

struct State {
    globals: GlobalList,
    compositor: wl_compositor::WlCompositor,
    shm: wl_shm::WlShm,
    viewporter: wp_viewporter::WpViewporter,
    layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,

    outputs: Vec<OutputInfo>,
    displays: Vec<Display>,
}

impl State {
    fn new(
        globals: GlobalList,
        compositor: wl_compositor::WlCompositor,
        shm: wl_shm::WlShm,
        viewporter: wp_viewporter::WpViewporter,
        layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
    ) -> Self {
        Self {
            globals,
            compositor,
            shm,
            viewporter,
            layer_shell,
            outputs: Vec::new(),
            displays: Vec::new(),
        }
    }

    fn bind_initial_outputs(&mut self, qh: &QueueHandle<Self>) {
        self.globals.contents().with_list(|list| {
            for global in list {
                if global.interface == wl_output::WlOutput::interface().name {
                    let output: wl_output::WlOutput =
                        self.globals
                            .registry()
                            .bind(global.name, global.version, qh, ());
                    self.outputs.push(OutputInfo::new(output, global.name));
                }
            }
        });
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                println!("Global added: {} [{}(v{})]", name, interface, version);
                if interface == wl_output::WlOutput::interface().name {
                    let output: wl_output::WlOutput = proxy.bind(name, version, qhandle, ());
                    state.outputs.push(OutputInfo::new(output, name));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                println!("Global removed: {}", name);
                if let Some(i) = state.outputs.iter().position(|o| o.output_name == name) {
                    state.outputs.swap_remove(i);
                } else if let Some(i) = state.displays.iter().position(|o| o.output_name == name) {
                    state.outputs.swap_remove(i);
                }
            }
            _ => (),
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_compositor::WlCompositor,
        _event: <wl_compositor::WlCompositor as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm::WlShm,
        event: <wl_shm::WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_shm::Event::Format { format } => {
                println!("wl_shm: format {:?} available", format)
            }
            _ => (),
        }
    }
}

impl Dispatch<wp_viewporter::WpViewporter, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wp_viewporter::WpViewporter,
        _event: <wp_viewporter::WpViewporter as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _event: <zwlr_layer_shell_v1::ZwlrLayerShellV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: <wl_surface::WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_region::WlRegion,
        _event: <wl_region::WlRegion as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_viewport::WpViewport, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wp_viewport::WpViewport,
        _event: <wp_viewport::WpViewport as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_shm_pool::WlShmPool,
        _event: <wl_shm_pool::WlShmPool as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for State {
    fn event(
            _state: &mut Self,
            _proxy: &wl_buffer::WlBuffer,
            _event: <wl_buffer::WlBuffer as Proxy>::Event,
            _data: &(),
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {}
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                if let Some(i) = state
                    .displays
                    .iter()
                    .position(|d| d.layer_surface.id() == proxy.id())
                {
                    let display = &mut state.displays[i];
                    display.width = width;
                    display.height = height;
                    display.ack_serial = serial;
                    display.needs_ack = true;
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_output::WlOutput,
        event: <wl_output::WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Geometry { x, y, .. } => {
                if let Some(i) = state
                    .outputs
                    .iter()
                    .position(|o| o.output.id() == proxy.id())
                {
                    state.outputs[i].x = Some(x);
                    state.outputs[i].y = Some(y);
                }
            }
            wl_output::Event::Name { name } => {
                if let Some(i) = state
                    .outputs
                    .iter()
                    .position(|o| o.output.id() == proxy.id())
                {
                    state.outputs[i].name = Some(name);
                }
            }
            wl_output::Event::Description { description } => {
                if let Some(i) = state
                    .outputs
                    .iter()
                    .position(|o| o.output.id() == proxy.id())
                {
                    state.outputs[i].desc = Some(description);
                }
            }
            wl_output::Event::Done => {
                if let Some(i) = state
                    .outputs
                    .iter()
                    .position(|o| o.output.id() == proxy.id())
                {
                    let monitor = state.outputs.remove(i);
                    println!(
                        "Monitor {} sent Done with (x: {:?}, y: {:?}, name: {:?}, desc: {:?})",
                        monitor.output.id(),
                        monitor.x,
                        monitor.y,
                        monitor.name,
                        monitor.desc
                    );
                    let display = Display::new(state, monitor, qhandle);
                    state.displays.push(display);
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
struct OutputInfo {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub output: wl_output::WlOutput,
    pub output_name: u32,
}

impl OutputInfo {
    fn new(output: wl_output::WlOutput, output_name: u32) -> Self {
        Self {
            output,
            output_name,
            x: None,
            y: None,
            name: None,
            desc: None,
        }
    }
}

struct Display {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub output: wl_output::WlOutput,
    pub output_name: u32,

    x: i32,
    y: i32,
    width: u32,
    height: u32,

    wl_surface: wl_surface::WlSurface,
    viewport: wp_viewport::WpViewport,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,

    ack_serial: u32,
    needs_ack: bool,
    pub configured: bool,

    dirty: bool,
    image: DynamicImage,
}

impl Display {
    fn new(state: &mut State, output: OutputInfo, qh: &QueueHandle<State>) -> Self {
        let OutputInfo {
            name,
            desc,
            x,
            y,
            output,
            output_name,
        } = output;

        let State {
            compositor,
            viewporter,
            layer_shell,
            ..
        } = state;

        let (w, h) = (4u32, 4u32);

        let wl_surface = compositor.create_surface(qh, ());
        let region = compositor.create_region(qh, ());

        wl_surface.set_input_region(Some(&region));
        region.destroy();

        let layer_surface = layer_shell.get_layer_surface(
            &wl_surface,
            Some(&output),
            zwlr_layer_shell_v1::Layer::Background,
            String::from("waypaper-engine"),
            qh,
            (),
        );

        let viewport = viewporter.get_viewport(&wl_surface, qh, ());

        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_margin(0, 0, 0, 0);
        layer_surface.set_keyboard_interactivity(0);

        wl_surface.commit();

        println!(
            "created display - name: {}, surface: {}, layer surface: {}",
            output.id(),
            wl_surface.id(),
            layer_surface.id()
        );

        Self {
            name,
            desc,
            output,
            output_name,
            x: x.unwrap_or(0),
            y: y.unwrap_or(0),
            width: w,
            height: h,
            wl_surface,
            viewport,
            layer_surface,
            ack_serial: 0,
            needs_ack: false,
            configured: false,
            dirty: false,
            image: DynamicImage::new(w, h, image::ColorType::Rgba8),
        }
    }

    fn render_image(
        &mut self,
        shm: &mut wl_shm::WlShm,
        qh: &QueueHandle<State>,
        img: DynamicImage,
    ) {
        self.image = img.resize_to_fill(
            self.width,
            self.height,
            image::imageops::FilterType::CatmullRom,
        );

        let rgba = self.image.to_rgba8();
        let stride = (4 * self.width) as i32;
        let size = (stride as usize) * (self.height as usize);

        let mut pixels: Vec<u8> = Vec::with_capacity(size);
        for chunk in rgba.chunks_exact(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = 255u8;

            pixels.push(b);
            pixels.push(g);
            pixels.push(r);
            pixels.push(a);
        }

        let mut tmpfile = tempfile().expect("Failed to create tempfile for shm");
        tmpfile.set_len(size as u64).ok();
        tmpfile.write_all(&pixels).expect("write failed");
        // tempfile.write_all(&vec![0u8; 0]).ok();

        let mut mmap = unsafe { MmapMut::map_mut(&tmpfile).expect("mmap failed lmao") };
        mmap[..size].copy_from_slice(&pixels);
        mmap.flush().ok();

        let shm_pool = shm.create_pool(tmpfile.as_fd(), size as i32, qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            self.width as i32,
            self.height as i32,
            stride,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );
        shm_pool.destroy();

        println!(
            "create_and_damage_buffer: attaching buffer {}x{} id={}",
            self.width,
            self.height,
            buffer.id()
        );

        self.wl_surface.attach(Some(&buffer), 0, 0);
        self.wl_surface.damage(0, 0, self.width as i32, self.height as i32);
        self.wl_surface.commit();

        std::mem::forget(mmap);
        std::mem::forget(tmpfile);
    }
}

fn main() -> Result<()> {
    let image_path = PathBuf::from(
        env::args()
            .nth(1)
            .expect("Failed to parse arguments, expected image path"),
    );
    let img = image::open(&image_path)?;

    let conn = Connection::connect_to_env()?;

    let (globals, mut event_queue) = registry_queue_init::<State>(&conn)?;
    let qh = event_queue.handle();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=5, ())?;
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ())?;
    let viewporter: wp_viewporter::WpViewporter = globals.bind(&qh, 1..=1, ())?;
    let layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1 = globals.bind(&qh, 1..=1, ())?;

    let mut state = State::new(globals, compositor, shm, viewporter, layer_shell);
    state.bind_initial_outputs(&qh);

    loop {
        event_queue.blocking_dispatch(&mut state)?;

        for display in state.displays.iter_mut() {
            if display.needs_ack {
                display.layer_surface.ack_configure(display.ack_serial);
                display.needs_ack = false;
                display.dirty = true;
            }
        }

        for display in state.displays.iter_mut() {
            if display.dirty {
                display.render_image(&mut state.shm, &qh, img.clone());
                display.dirty = false;
            }
        }
    }
}
