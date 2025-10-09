mod wayland;

use anyhow::{Context, Result};
use memmap2::MmapMut;
use std::{env, fmt::format, path::PathBuf, sync::Arc, time::Duration};
use waybackend::{Global, objman, types::ObjectId};

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

    monitors: Vec<Output>,

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

        println!("{:#?}", monitors);

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

    Ok(())
}
