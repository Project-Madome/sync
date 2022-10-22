mod config;
mod container;
pub mod error;
mod registry;

pub use error::{Error, SendError};

pub type Result<T> = std::result::Result<T, crate::Error>;

#[macro_export]
macro_rules! none_to_continue {
    ($r:expr) => {
        match $r {
            Some(r) => r,
            None => continue,
        }
    };

    ($r:expr, $l:tt) => {
        match $r {
            Some(r) => r,
            None => continue $l,
        }
    };
}
