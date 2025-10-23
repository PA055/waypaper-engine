mod wayland;

use crate::wayland::{
    wl_buffer, wl_callback, wl_compositor, wl_display, wl_output, wl_region, wl_registry, wl_shm,
    wl_shm_pool, wl_surface, wp_viewport, wp_viewporter, zwlr_layer_shell_v1,
    zwlr_layer_surface_v1,
};
use anyhow::{Context, Result};
use image::{ColorType, DynamicImage};
use memmap2::MmapMut;
use std::{env, io::Write, os::fd::AsRawFd, path::PathBuf};
use tempfile::tempfile;
use waybackend::{
    Waybackend, match_enum_with_interface,
    objman::{self, ObjectManager},
    types::ObjectId,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum WaylandObject {
    Display,
    Registry,
    Callback,
    Compositor,
    Shm,
    ShmPool,
    Buffer,
    Surface,
    Region,
    Output,
    Viewporter,
    Viewport,

    LayerShell,
    LayerSurface,
}

struct EngineBackend {
    backend: waybackend::Waybackend,
    objman: objman::ObjectManager<WaylandObject>,
    _registry: ObjectId,
    compositor: ObjectId,
    shm: ObjectId,
    viewporter: ObjectId,
    layer_shell: ObjectId,

    pending_monitors: Vec<OutputInfo>,
    outputs: Vec<Output>,
}

impl EngineBackend {
    fn new(backend: waybackend::Waybackend, objman: objman::ObjectManager<WaylandObject>) -> Self {
        let _registry = objman
            .get_first(WaylandObject::Registry)
            .expect("Missing wayland registry.");
        let compositor = objman
            .get_first(WaylandObject::Compositor)
            .expect("Missing wayland compositor.");
        let shm = objman
            .get_first(WaylandObject::Shm)
            .expect("Missing wayland shm.");
        let layer_shell = objman
            .get_first(WaylandObject::LayerShell)
            .expect("Cannot run without zwlr_layer_shell support.");
        let viewporter = objman
            .get_first(WaylandObject::Viewporter)
            .expect("wp_viewporter is required,");

        let monitors: Vec<OutputInfo> = objman
            .get_all(WaylandObject::Output)
            .map(|output| OutputInfo::new(output))
            .collect();

        if monitors.is_empty() {
            panic!("No displays available.");
        }

        Self {
            backend,
            objman,
            _registry,
            compositor,
            shm,
            viewporter,
            layer_shell,
            pending_monitors: monitors,
            outputs: Vec::new(),
        }
    }
}

impl wl_display::EvHandler for EngineBackend {
    fn error(&mut self, _sender_id: ObjectId, _object_id: ObjectId, code: u32, message: &str) {
        println!("oops error in display :skull: {}: {}", code, message)
    }

    fn delete_id(&mut self, _sender_id: ObjectId, id: u32) {
        println!("delete_id {}", id);
        self.objman.remove(id);
    }
}

impl wl_registry::EvHandler for EngineBackend {
    fn global(&mut self, _sender_id: ObjectId, name: u32, interface: &str, version: u32) {
        println!("Global [{}] {} (v{})", name, interface, version);
    }

    fn global_remove(&mut self, _sender_id: ObjectId, name: u32) {
        println!("Global removed: {}", name)
    }
}

impl wl_callback::EvHandler for EngineBackend {
    fn done(&mut self, _sender_id: ObjectId, callback_data: u32) {
        println!(
            "umm the callback ran and idk what to do for it so here: {}",
            callback_data
        );
    }
}

impl wl_compositor::EvHandler for EngineBackend {}

impl wl_shm::EvHandler for EngineBackend {
    fn format(&mut self, _sender_id: ObjectId, _format: wl_shm::Format) {
        // println!("shm format {:?}", _format);
        // no-op
    }
}

impl wl_shm_pool::EvHandler for EngineBackend {}

impl wl_buffer::EvHandler for EngineBackend {
    fn release(&mut self, _sender_id: ObjectId) {
        println!("rip buffer {} ig we leak it now :broken_heart:", _sender_id);
    }
}

impl wl_surface::EvHandler for EngineBackend {
    fn enter(&mut self, _sender_id: ObjectId, output: ObjectId) {
        println!("Output {}: Surface {} Enter", output, _sender_id);
    }

    fn leave(&mut self, _sender_id: ObjectId, output: ObjectId) {
        println!("Output {}: Surface {} Leave", output, _sender_id);
    }

    fn preferred_buffer_scale(&mut self, _sender_id: ObjectId, _factor: i32) {
        // println!("surface scale factor: {}", _factor);
        // no-op
    }

    fn preferred_buffer_transform(
        &mut self,
        _sender_id: ObjectId,
        _transform: wl_output::Transform,
    ) {
        // no-op
    }
}

impl wl_region::EvHandler for EngineBackend {}

impl wl_output::EvHandler for EngineBackend {
    fn geometry(
        &mut self,
        _sender_id: ObjectId,
        _x: i32,
        _y: i32,
        _physical_width: i32,
        _physical_height: i32,
        _subpixel: wl_output::Subpixel,
        _make: &str,
        _model: &str,
        _transform: wl_output::Transform,
    ) {
        // println!(
        //     "Output {} geometry\n  x: {}\n  y: {}\n  pw: {}\n  ph: {}\n  make: {}\n model: {}",
        //     _sender_id, _x, _y, _physical_width, _physical_height, _make, _model
        // );
        // no-op
    }

    fn mode(
        &mut self,
        _sender_id: ObjectId,
        _flags: wl_output::Mode,
        _width: i32,
        _height: i32,
        _refresh: i32,
    ) {
        // println!(
        //     "Output {} mode\n  flags: {}\n  w: {}\n  h: {}\n  r: {}",
        //     _sender_id,
        //     _flags.bits(),
        //     _width,
        //     _height,
        //     _refresh
        // );
        // no-op
    }

    fn done(&mut self, sender_id: ObjectId) {
        if let Some(i) = self
            .pending_monitors
            .iter()
            .position(|o| o.output == sender_id)
        {
            let monitor_info = self.pending_monitors.remove(i);
            println!(
                "Output {} done:\n  name: {:?}\n  desc: {:?}",
                sender_id, monitor_info.name, monitor_info.desc
            );
            let output = Output::new(self, monitor_info);
            self.outputs.push(output);
        }
    }

    fn scale(&mut self, _sender_id: ObjectId, _factor: i32) {
        // println!("Output {}: scale - {}", _sender_id, _factor);
        // no-op
    }

    fn name(&mut self, sender_id: ObjectId, name: &str) {
        // println!("Output {}: name - {}", sender_id, name);
        for info in self.pending_monitors.iter_mut() {
            if info.output == sender_id {
                info.name = Some(name.to_string());
            }
        }
    }

    fn description(&mut self, sender_id: ObjectId, description: &str) {
        // println!("Output {}: desc - {}", sender_id, description);
        for info in self.pending_monitors.iter_mut() {
            if info.output == sender_id {
                info.desc = Some(description.to_string());
            }
        }
    }
}

impl wp_viewporter::EvHandler for EngineBackend {}

impl wp_viewport::EvHandler for EngineBackend {}

impl zwlr_layer_shell_v1::EvHandler for EngineBackend {}

impl zwlr_layer_surface_v1::EvHandler for EngineBackend {
    fn configure(&mut self, sender_id: ObjectId, serial: u32, width: u32, height: u32) {
        if let Some(pos) = self
            .outputs
            .iter()
            .position(|o| o.layer_surface == sender_id)
        {
            let output = &mut self.outputs[pos];
            output.width = width as i32;
            output.height = height as i32;
            output.ack_serial = serial;
            output.configured = true;

            println!(
                "configure: layer_surface={} serial={} -> will ack and create buffer",
                sender_id, serial
            );

            zwlr_layer_surface_v1::req::ack_configure(&mut self.backend, sender_id, serial)
                .unwrap();
            println!(
                "ack sent; creating buffer for output {}",
                output.output
            );

            zwlr_layer_surface_v1::req::set_size(
                &mut self.backend,
                output.layer_surface,
                width,
                height,
            )
            .unwrap();
            println!(
                "create_and_damage_buffer: attaching buffer {}x{} to surface {}",
                width, height, output.wl_surface
            );
            output.create_and_damage_buffer(
                &mut self.backend,
                &mut self.objman,
                self.shm,
                output.image.clone(),
            );
            println!("create_and_damage_buffer: committed. flushing...");
            self.backend.flush().ok();
        }
    }

    fn closed(&mut self, _sender_id: ObjectId) {
        println!("Layer Surface {} closed", _sender_id)
        // destroy output and clean up os should do this :3 so we do this last
    }
}

#[derive(Debug)]
struct OutputInfo {
    pub name: Option<String>,
    pub desc: Option<String>,

    pub output: ObjectId,
}

impl OutputInfo {
    fn new(output: ObjectId) -> Self {
        Self {
            name: None,
            desc: None,
            output,
        }
    }
}

struct Output {
    name: Option<String>,
    desc: Option<String>,
    output: ObjectId,

    wl_surface: ObjectId,
    viewport: ObjectId,
    layer_surface: ObjectId,

    width: i32,
    height: i32,

    ack_serial: u32,
    needs_ack: bool,

    pub configured: bool,
    dirty: bool,

    image: DynamicImage,
}

impl Output {
    fn new(engine: &mut EngineBackend, monitor_info: OutputInfo) -> Self {
        let EngineBackend {
            backend,
            objman,
            viewporter,
            compositor,
            // shm,
            layer_shell,
            ..
        } = engine;

        let OutputInfo {
            name,
            desc,
            output,
        } = monitor_info;

        let wl_surface = objman.create(WaylandObject::Surface);
        wl_compositor::req::create_surface(backend, *compositor, wl_surface).unwrap();

        let region = objman.create(WaylandObject::Region);
        wl_compositor::req::create_region(backend, *compositor, region).unwrap();

        wl_surface::req::set_input_region(backend, wl_surface, Some(region)).unwrap();
        wl_region::req::destroy(backend, region).unwrap();

        let layer_surface = objman.create(WaylandObject::LayerSurface);
        zwlr_layer_shell_v1::req::get_layer_surface(
            backend,
            *layer_shell,
            layer_surface,
            wl_surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::background,
            "waypaper-engine",
        )
        .unwrap();

        let viewport = objman.create(WaylandObject::Viewport);
        wp_viewporter::req::get_viewport(backend, *viewporter, viewport, wl_surface).unwrap();

        zwlr_layer_surface_v1::req::set_anchor(
            backend,
            layer_surface,
            zwlr_layer_surface_v1::Anchor::TOP
                | zwlr_layer_surface_v1::Anchor::LEFT
                // | zwlr_layer_surface_v1::Anchor::BOTTOM
                // | zwlr_layer_surface_v1::Anchor::RIGHT,
        )
        .unwrap();
        zwlr_layer_surface_v1::req::set_exclusive_zone(backend, layer_surface, -1).unwrap();
        zwlr_layer_surface_v1::req::set_margin(backend, layer_surface, 0, 0, 0, 0).unwrap();
        zwlr_layer_surface_v1::req::set_keyboard_interactivity(backend, layer_surface, 0).unwrap();
        zwlr_layer_surface_v1::req::set_size(backend, layer_surface, 200, 200).unwrap();

        wl_surface::req::commit(backend, wl_surface).unwrap();

        println!(
            "created output: name: {}, surface: {}, layer surface: {}",
            output, wl_surface, layer_surface
        );
        Self {
            name,
            desc,
            output,
            wl_surface,
            viewport,
            layer_surface,
            width: 200,
            height: 200,
            ack_serial: 0,
            needs_ack: false,
            configured: false,
            dirty: false,
            image: DynamicImage::new(4, 4, ColorType::Rgba8),
        }

        // let im = DynamicImage::new_rgba8(200, 200);
        // out.create_and_damage_buffer(backend, objman, *shm, im);
    }

    fn create_and_damage_buffer(
        &mut self,
        backend: &mut Waybackend,
        objman: &mut ObjectManager<WaylandObject>,
        shm: ObjectId,
        image: DynamicImage,
    ) {
        let width = self.width.max(1) as u32;
        let height = self.height.max(1) as u32;
        self.image = image;
        if self.image.width() != width || self.image.height() != height {
            self.image =
                self.image
                    .resize_exact(width, height, image::imageops::FilterType::CatmullRom);
        }

        let rgba = self.image.to_rgba8();
        let stride = (4 * width) as i32;
        let size = (stride as usize) * (height as usize);

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

        let mut tempfile = tempfile().expect("Failed to create tempfile for shm");
        tempfile.set_len(size as u64).ok();
        tempfile.write_all(&pixels).expect("write failed");
        // tempfile.write_all(&vec![0u8; 0]).ok();

        let mut mmap = unsafe { MmapMut::map_mut(&tempfile).expect("mmap failed lmao") };
        mmap[..size].copy_from_slice(&pixels);
        mmap.flush().ok();

        let shm_pool = objman.create(WaylandObject::ShmPool);
        wl_shm::req::create_pool(backend, shm, shm_pool, &tempfile.as_raw_fd(), size as i32)
            .expect("failed to create shm_pool");

        let wl_buffer = objman.create(WaylandObject::Buffer);
        wl_shm_pool::req::create_buffer(
            backend,
            shm_pool,
            wl_buffer,
            0,
            width as i32,
            height as i32,
            stride,
            wl_shm::Format::argb8888,
        )
        .expect("failed to create buffer");

        wl_shm_pool::req::destroy(backend, shm_pool)
            .expect("failed to destroy shm_pool (idk if this is needed)");

        println!(
            "create_and_damage_buffer: attaching buffer {}x{} id={}",
            width,
            height,
            wl_buffer
        );
        wl_surface::req::attach(backend, self.wl_surface, Some(wl_buffer), 0, 0)
            .expect("failed to attach buffer");
        wl_surface::req::damage(backend, self.wl_surface, 0, 0, width as i32, height as i32)
            .expect("failed to damage surface");
        wl_surface::req::commit(backend, self.wl_surface).expect("failed to commit surface");
        backend.flush().ok();

        // i should store the tempfile and mmap but i kinda wanna do that with better structure so
        // we just forget the 2 for now so rust doesnt clean it up
        std::mem::forget(mmap);
        std::mem::forget(tempfile);
    }
}

fn main() -> Result<()> {
    let image_path: PathBuf = PathBuf::from(
        env::args()
            .nth(1)
            .with_context(|| "Failed to parse arguments, expected image path")?,
    );

    let img = image::open(&image_path)
        .with_context(|| format!("Failed to open image at {}", image_path.display()))?;

    let (mut backend, mut objman, mut reciever) = waybackend::connect(WaylandObject::Display)
        .with_context(|| "Could not connect to wayland server.")?;

    let registry = objman.create(WaylandObject::Registry);
    let callback = objman.create(WaylandObject::Callback);
    let (globals, delete_callback) =
        waybackend::roundtrip(&mut backend, &mut reciever, registry, callback).unwrap();

    if delete_callback {
        objman.remove(callback.get().get());
    }

    {
        use WaylandObject::*;
        use wayland::*;
        waybackend::bind_globals!(
            backend,
            objman,
            registry,
            globals,
            (wl_compositor, Compositor),
            (wl_shm, Shm),
            (wp_viewporter, Viewporter),
            (zwlr_layer_shell_v1, LayerShell),
            (wl_output, Output)
        );
    };

    let mut engine = EngineBackend::new(backend, objman);
    engine.backend.flush()?;

    // let mut dirty = true;
    loop {
        let mut msg = reciever.recv(&engine.backend.wayland_fd)?;
        while msg.has_next()? {
            let sender = engine.objman.get(msg.sender_id()).unwrap();
            match_enum_with_interface!(
                engine,
                sender,
                msg,
                (WaylandObject::Display, wl_display),
                (WaylandObject::Registry, wl_registry),
                (WaylandObject::Callback, wl_callback),
                (WaylandObject::Compositor, wl_compositor),
                (WaylandObject::Shm, wl_shm),
                (WaylandObject::ShmPool, wl_shm_pool),
                (WaylandObject::Buffer, wl_buffer),
                (WaylandObject::Surface, wl_surface),
                (WaylandObject::Region, wl_region),
                (WaylandObject::Output, wl_output),
                (WaylandObject::Viewporter, wp_viewporter),
                (WaylandObject::Viewport, wp_viewport),
                (WaylandObject::LayerShell, zwlr_layer_shell_v1),
                (WaylandObject::LayerSurface, zwlr_layer_surface_v1),
            );
        }

        /*
        if dirty {
            for output in engine.outputs.iter_mut() {
                output.create_and_damage_buffer(
                    &mut engine.backend,
                    &mut engine.objman,
                    engine.shm,
                    img.clone(),
                );
            }
            dirty = false;
        }
        */
    }
}
