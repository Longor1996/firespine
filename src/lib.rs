#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

// Global Imports
pub use log::{info, warn, debug};
pub use std::cell::RefCell;
pub use std::sync::Arc;
pub use std::any::TypeId;
pub use downcast_rs::{Downcast, DowncastSync};
pub use futures::future::BoxFuture;

pub mod comp;
pub use comp::*;

pub mod event;
pub use event::*;

pub mod node;
pub use node::*;

pub mod thunk;
pub use thunk::*;

pub mod ctx;
pub use ctx::*;

/// The backbone: A 'hierarchy' of named nodes.
/// 
/// It is required to call the `update`-method as part of a mainloop.
pub struct Backbone {
    /// Stack of active nodes.
    nodes: Nodes,
    
    /// Queue of thunks.
    thunks: Thunks,
}

// Constructors.
impl Backbone {
    /// Creates a new backbone instance.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Creates a new backbone instance.
    pub fn from_obj<N: NodeHandler + 'static>(root_handler: N) -> Self {
        let root_handler = Box::new(root_handler);
        Self::from_box(("/".into(), root_handler))
    }
    
    /// Creates a new backbone instance.
    pub fn from_box(root_handler: NodeHandlerBox) -> Self {
        Self {
            nodes: vec![root_handler],
            thunks: Thunks::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Backbone {
    fn default() -> Self {
        Self::from_obj(node::empty::EmptyEventHandler)
    }
}

impl<N: NodeHandler + 'static> From<N> for Backbone {
    fn from(node: N) -> Self {
        Self::from_obj(node)
    }
}

// Lööp.
impl Backbone {
    /// Backbone update function; to be called repeatedly in a lööp.
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.process_thunks()
    }
}
