//! Components
use crate::*;

/// A component.
pub trait NodeComponent: Downcast {
    /// Checks if this component has the given type-id.
    fn is_of_type(&self, ctype: TypeId) -> bool {
        self.type_id() == ctype
    }
}

// Automatic impl for sized components.
impl<C: Sized + 'static> NodeComponent for C where C: Sized {}

/// A component that is also Send/Sync.
pub trait NodeComponentSync: NodeComponent + DowncastSync + Send + Sync {
    //
}

// Automatic impl for Send/Sync components.
impl<C: NodeComponent> NodeComponentSync for C where C: Send + Sync {}

/// A box holding a NodeComponent instance.
pub type NodeComponentBox = Box<dyn NodeComponent>;

// Downcast-Impl.
use downcast_rs::impl_downcast;
impl_downcast!(NodeComponent);
impl_downcast!(NodeComponentSync);
