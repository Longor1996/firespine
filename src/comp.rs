//! Components
use crate::*;

/// A component.
pub trait NodeComponent: Downcast {
    /// Checks if this component has the given type-id.
    fn is_of_type(&self, ctype: TypeId) -> bool {
        self.type_id() == ctype
    }
    
    /// Returns the type-id for this component.
    fn get_component_type_id(&self) -> TypeId {
        self.type_id()
    }
    
    /// Returns a internal name for the component.
    fn get_component_name(&self) -> &str;
}

// Automatic impl for sized components.
impl<C: Sized + 'static> NodeComponent for C where C: Sized {
    fn get_component_name(&self) -> &str {
        std::any::type_name::<C>()
    }
}

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
