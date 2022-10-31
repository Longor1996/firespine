#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

// --- Global Imports
pub(crate) use std::cell::RefCell;
pub(crate) use std::sync::Arc;
pub(crate) use std::any::TypeId;
pub(crate) use log::{info, warn, debug};
pub(crate) use downcast_rs::{Downcast, DowncastSync};

// --- Public Prelude
/// API Prelude
pub mod prelude {
    pub use futures::channel::oneshot::channel as oneshot_channel;
    pub use futures::channel::oneshot::Sender as OneshotSender;
    pub use std::cell::RefCell;
    pub use std::sync::Arc;
    pub use crate::comp::{NodeComponent, NodeComponentSync};
    pub use crate::event::{Event, EventWrapper, EventPhase};
    pub use crate::node::{NodeHandler, NodeHandlerRequestRes, NodeHandlerRequest, cstore::CStoreEventHandler};
    pub use crate::ctx::{OuterNodeContext, NodeContext};
    pub use crate::Backbone;
}

// --- Re-exports of used libraries.
pub use log;
pub use futures;
pub use downcast_rs;

// --- Modules
pub mod comp;
pub mod event;
pub mod node;
pub mod thunk;
pub mod ctx;

// --- Internal Prelude
pub(crate) use comp::*;
pub(crate) use event::*;
pub(crate) use node::*;
pub(crate) use thunk::*;
pub(crate) use ctx::*;

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
    pub fn from_obj<N: NodeHandler + 'static>(root_handler: N) -> Self {
        let root_handler = Box::new(root_handler);
        Self::from_box(root_handler)
    }
    
    /// Creates a new backbone instance.
    pub fn from_box(root_handler: NodeHandlerBox) -> Self {
        Self {
            nodes: vec![NamedNodeHandlerBox {
                name: "/".into(),
                node: root_handler
            }],
            thunks: Thunks::default(),
        }
    }
    
    /// Wraps the current root with the given handler to form a cascade.
    pub fn cascade<N: NodeHandler + 'static>(self, handler: N) -> Self {
        let Self { mut nodes, .. } = self;
        if nodes.len() != 1 {panic!("can only cascade when the root is the sole node")}
        let node = nodes.remove(0);
        let handler = Box::new(handler);
        
        Self {
            nodes: vec![NamedNodeHandlerBox {
                name: "/".into(),
                node: Box::new(cascade::CascadingEventHandler {
                    outer: NamedNodeHandlerBox {
                        name: "/".into(),
                        node: handler
                    },
                    inner: node
                })
            }],
            thunks: Thunks::default(),
        }
    }
}

impl<N: NodeHandler + 'static> From<N> for Backbone {
    fn from(node: N) -> Self {
        Self::from_obj(node)
    }
}

// This impl. is technically useless!
#[allow(clippy::derivable_impls)]
impl Default for Backbone {
    fn default() -> Self {
        Self::from_obj(node::empty::EmptyEventHandler)
    }
}

// Lööp.
impl Backbone {
    /// Backbone update function; to be called repeatedly in a lööp.
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.process_thunks()
    }
}
