mod wayland;

use crate::wayland::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use image::DynamicImage;
use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_display, wl_output, wl_region, wl_registry, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

use anyhow::Result;
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
};

struct State {
    outputs: Vec<OutputInfo>,
}

impl State {
    fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    fn bind_initial_outputs(&mut self, globals: &GlobalList, qh: &QueueHandle<Self>) {
        globals.contents().with_list(|list| {
            for global in list {
                if global.interface == wl_output::WlOutput::interface().name {
                    let output: wl_output::WlOutput =
                        globals
                            .registry()
                            .bind(global.name, global.version, &qh, ());
                    self.outputs.push(OutputInfo::new(output));
                }
            }
        });
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match _event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                println!("Global added: {} [{}(v{})]", name, interface, version);
            }
            wl_registry::Event::GlobalRemove { name } => {
                println!("Global removed: {}", name)
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

impl Dispatch<wl_output::WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_output::WlOutput,
        event: <wl_output::WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
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
                    let monitor = &state.outputs[i];
                    println!(
                        "Monitor {} sent Done with (x: {:?}, y: {:?}, name: {:?}, desc: {:?})",
                        monitor.output.id(),
                        monitor.x,
                        monitor.y,
                        monitor.name,
                        monitor.desc
                    )
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
}

impl OutputInfo {
    fn new(output: wl_output::WlOutput) -> Self {
        Self {
            output,
            x: None,
            y: None,
            name: None,
            desc: None,
        }
    }
}

struct Output {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub output: u32,

    x: i32,
    y: i32,
    width: i32,
    height: i32,

    ack_serial: u32,
    needs_ack: bool,
    pub configured: bool,

    dirty: bool,
    image: DynamicImage,
}

fn main() -> Result<()> {
    let conn = Connection::connect_to_env()?;
    let mut state = State::new();

    let (globals, mut event_queue) = registry_queue_init::<State>(&conn)?;
    let qh = event_queue.handle();

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=5, ())?;
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ())?;
    let viewporter: wp_viewporter::WpViewporter = globals.bind(&qh, 1..=1, ())?;
    let layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1 = globals.bind(&qh, 1..=1, ())?;
    state.bind_initial_outputs(&globals, &qh);

    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}
