pub use root_registry::RootRegistry;

mod root_registry {
    use sai::{component_registry, Component};

    use crate::{config::Config, container};

    component_registry!(
        RootRegistry,
        [
            container::Token,
            container::Token,
            container::About,
            container::Channel,
            container::Image,
            container::Nozomi,
            container::Sync,
            container::Progress,
            Config
        ]
    );
}
