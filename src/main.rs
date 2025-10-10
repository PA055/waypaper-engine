mod wayland;

use anyhow::{Context, Result, bail};
use memmap2::MmapMut;
use std::{env, os::fd::AsRawFd, path::PathBuf};
use tempfile::tempfile;
use waybackend::{match_enum_with_interface, objman, types::ObjectId};

use crate::wayland::{
    wl_buffer, wl_callback, wl_compositor, wl_display, wl_output, wl_region, wl_registry, wl_shm,
    wl_shm_pool, wl_surface, zwlr_layer_shell_v1, zwlr_layer_surface_v1,
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

    LayerShell,
    LayerSurface,
}

struct EngineBackend {
    backend: waybackend::Waybackend,
    objman: objman::ObjectManager<WaylandObject>,
    registry: ObjectId,
    compositor: ObjectId,
    shm: ObjectId,
    layer_shell: ObjectId,

    monitors: Vec<Output>, // todo: switch to a queue between uninited monitors and wallpapers like 

    image: image::DynamicImage,
}

impl EngineBackend {
    fn new(
        backend: waybackend::Waybackend,
        objman: objman::ObjectManager<WaylandObject>,
        image: image::DynamicImage,
    ) -> Self {
        let registry = objman
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

        let monitors: Vec<Output> = objman
            .get_all(WaylandObject::Output)
            .map(|output| Output::new(output))
            .collect();

        if monitors.is_empty() {
            panic!("No displays available.");
        }

        Self {
            backend,
            objman,
            registry,
            compositor,
            shm,
            layer_shell,
            monitors,
            image,
        }
    }

    fn create_shm_buffer(engine: &mut Self, width: u32, height: u32) -> Result<ObjectId> {
        if width == 0 || height == 0 {
            bail!("Invalid size for buffer");
        }

        let rgba = engine
            .image
            .resize_to_fill(width, height, image::imageops::FilterType::CatmullRom)
            .to_rgba8();

        let stride = (width as usize) * 4;
        let size = stride * (height as usize);

        let file = tempfile()?;
        file.set_len(size as u64);

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        mmap.copy_from_slice(rgba.as_raw());

        mmap.flush();

        let shm_pool = engine.objman.create(WaylandObject::ShmPool);
        let fd = file.as_raw_fd();
        wl_shm::req::create_pool(&mut engine.backend, engine.shm, shm_pool, &fd, size as i32)?;

        let wl_buffer = engine.objman.create(WaylandObject::Buffer);
        wl_shm_pool::req::create_buffer(
            &mut engine.backend,
            shm_pool,
            wl_buffer,
            0,
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::rgba8888,
        )?;

        wl_shm_pool::req::destroy(&mut engine.backend, shm_pool)?;

        Ok(wl_buffer)
    }

    fn init_wallpaper(&mut self, monitor_idx: usize) {
        !todo();
        // todo, just init everything
    }
}

impl wl_display::EvHandler for EngineBackend {
    fn error(&mut self, _sender_id: ObjectId, _object_id: ObjectId, code: u32, message: &str) {
        println!("oops error in display :skull: {}: {}", code, message)
    }

    fn delete_id(&mut self, _sender_id: ObjectId, id: u32) {
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
        // no-op
    }
}

impl wl_shm_pool::EvHandler for EngineBackend {}

impl wl_buffer::EvHandler for EngineBackend {
    fn release(&mut self, _sender_id: ObjectId) {
        println!("rip buffer ig we leak it now :broken_heart:");
    }
}

impl wl_surface::EvHandler for EngineBackend {
    fn enter(&mut self, _sender_id: ObjectId, output: ObjectId) {
        println!("Output: {}: Surface Enter", output);
    }

    fn leave(&mut self, _sender_id: ObjectId, output: ObjectId) {
        println!("Output: {}: Surface Leave", output);
    }

    fn preferred_buffer_scale(&mut self, _sender_id: ObjectId, _factor: i32) {
        // no-op
    }

    fn preferred_buffer_transform(&mut self, _sender_id: ObjectId, _transform: wl_output::Transform) {
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
        // no-op
    }

    fn done(&mut self, sender_id: ObjectId) {
        if let Some(output) = self.monitors.iter().position(|o| o.output == sender_id) {
            self.init_wallpaper(output);
        }
    }

    fn scale(&mut self, _sender_id: ObjectId, _factor: i32) {
        // no-op
    }

    fn name(&mut self, sender_id: ObjectId, name: &str) {
        for info in self.monitors.iter_mut() {
            if info.output == sender_id {
                info.name = Some(name.to_string());
            }
        }
    }

    fn description(&mut self, sender_id: ObjectId, description: &str) {
        for info in self.monitors.iter_mut() {
            if info.output == sender_id {
                info.desc = Some(description.to_string());
            }
        }
    }
}

impl zwlr_layer_shell_v1::EvHandler for EngineBackend {}

impl zwlr_layer_surface_v1::EvHandler for EngineBackend {
    fn configure(&mut self, sender_id: ObjectId, serial: u32, width: u32, height: u32) {
        // find the output using the layer_surface and set width and height
    }

    fn closed(&mut self, sender_id: ObjectId) {
        // destroy output and clean up os should do this :3 so we do this last
    }
}

#[derive(Debug)]
struct Output {
    pub name: Option<String>,
    pub desc: Option<String>,

    pub output: ObjectId,
    pub output_name: u32,
}

impl Output {
    fn new(output: ObjectId) -> Self {
        Self {
            name: None,
            desc: None,
            output,
            output_name: output.get().into(), // I have no idea if this works or not :skull:
        }
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
            (zwlr_layer_shell_v1, LayerShell),
            (wl_output, Output)
        );
    };

    let mut engine = EngineBackend::new(backend, objman, img);

    for monitor in engine.monitors.iter() {
        let wl_surface = engine.objman.create(WaylandObject::Surface);
        wl_compositor::req::create_surface(&mut engine.backend, engine.compositor, wl_surface)?;

        let region = engine.objman.create(WaylandObject::Region);
        wl_compositor::req::create_region(&mut engine.backend, engine.compositor, region)?;

        wl_surface::req::set_input_region(&mut engine.backend, wl_surface, Some(region))?;
        wl_region::req::destroy(&mut engine.backend, region)?;

        let layer_surface = engine.objman.create(WaylandObject::LayerSurface);
        zwlr_layer_shell_v1::req::get_layer_surface(
            &mut engine.backend,
            engine.layer_shell,
            layer_surface,
            wl_surface,
            Some(monitor.output),
            zwlr_layer_shell_v1::Layer::background,
            "waypaper-engine",
        )?;

        zwlr_layer_surface_v1::req::set_anchor(
            &mut engine.backend,
            layer_surface,
            zwlr_layer_surface_v1::Anchor::TOP
                | zwlr_layer_surface_v1::Anchor::BOTTOM
                | zwlr_layer_surface_v1::Anchor::LEFT
                | zwlr_layer_surface_v1::Anchor::RIGHT,
        )?;
        zwlr_layer_surface_v1::req::set_exclusive_zone(&mut engine.backend, layer_surface, -1)?;
        zwlr_layer_surface_v1::req::set_margin(&mut engine.backend, layer_surface, 0, 0, 0, 0)?;
        zwlr_layer_surface_v1::req::set_keyboard_interactivity(
            &mut engine.backend,
            layer_surface,
            0,
        )?;
        zwlr_layer_surface_v1::req::set_size(&mut engine.backend, layer_surface, 0, 0)?;

        wl_surface::req::commit(&mut engine.backend, wl_surface)?;
    }

    engine.backend.flush()?;

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
                (WaylandObject::LayerShell, zwlr_layer_shell_v1),
                (WaylandObject::LayerSurface, zwlr_layer_surface_v1),
            )
        }
    }
}
