mod wayland;

use anyhow::{Context, Result};
use image::GenericImageView;
use log::{error, info};
use memmap2::MmapMut;
use std::{env, fmt::format, path::PathBuf, sync::Arc, time::Duration};
use waybackend::{Global, objman, types::ObjectId};

struct EngineBackend {
    backend: waybackend::Waybackend,
    objman: objman::ObjectManager<WaylandObject>,
}

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

fn main() -> Result<()> {
    let image_path: PathBuf = PathBuf::from(
        env::args()
            .nth(1)
            .with_context(|| "Failed to parse arguments, expected image path")?,
    );

    let img = image::open(&image_path)
        .with_context(|| format!("Failed to open image at {}", image_path.display()))?;

    let (img, (img_w, img_h)) = (img.to_rgba8(), img.dimensions());

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

        println!(
            "{:#?}",
            globals
                .iter()
                .filter(|global| global.interface() == wl_output::NAME)
                .collect::<Vec<&Global>>()
        );
    };

    Ok(())
}
