pub mod wlr {
    pub mod layer_shell {
        use wayland_client;
        use wayland_client::protocol::*;
        use wayland_protocols::xdg::shell::client::*;

        pub mod __interfaces {
            use wayland_client::protocol::__interfaces::*;
            use wayland_protocols::xdg::shell::client::__interfaces::*;
            wayland_scanner::generate_interfaces!("protocols/wlr-layer-shell-unstable-v1.xml");
        }

        use self::__interfaces::*;
        wayland_scanner::generate_client_code!("protocols/wlr-layer-shell-unstable-v1.xml");
    }

    pub mod output_management {
        use wayland_client;
        use wayland_client::protocol::*;

        pub mod __interfaces {
            wayland_scanner::generate_interfaces!("protocols/wlr-output-management-unstable-v1.xml");
        }

        use self::__interfaces::*;
        wayland_scanner::generate_client_code!("protocols/wlr-output-management-unstable-v1.xml");
    }
}
