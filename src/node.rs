//! Nodes

use crate::*;

/// All user-logic and -state for a node is owned by it's associated handler.
pub trait NodeHandler: Downcast + std::fmt::Debug {
    /// Called when a new child-node is to be created.
    /// 
    /// **Note:**
    /// > The name of the child-node is *not* a full path, just a name.  
    /// > 
    /// > Use [`NodeContext::get_child_name`] to get the full path.
    fn handle_node_request<'e>(
        &'e mut self,
        _name: Arc<str>,
        _context: &'e mut NodeContext,
    ) -> NodeHandlerRequestRes {
        Err("Node has no children".into())
    }
    
    /// Called when the node receives an [`Event`] (wrapped in a [`EventWrapper`]).
    /// 
    /// i.e: Given a struct `MyEvent` that impls [`Event`] ...
    /// ```rust
    /// fn handle_event<'e>(
    ///     &'e mut self,
    ///     event: &'e mut EventWrapper,
    ///     context: &'e mut NodeContext,
    /// ) {
    ///     if let Some(my_event) = event.downcast_ref::<MyEvent>() {
    ///         // Handle the event! :D
    ///     }
    /// }
    /// ```
    fn handle_event<'e>(
        &'e mut self,
        event: &'e mut EventWrapper,
        context: &'e mut NodeContext,
    ) -> SubEvent {
        if !event.is_silent() {
            crate::debug!(
                "EVENT {} AT {}: {:?}",
                event.get_phase(),
                context.name,
                event.get_event()
            );
        }
        None
    }
    
    /// Called by [`NodeContext`] to fetch a component for a descendant node (or the backbone).
    fn get_comp(
        &self,
        _ctype: TypeId
    ) -> Option<&dyn NodeComponent> {None}
    
    /// Called by [`NodeContext`] to fetch a mutable component for a descendant node (or the backbone).
    fn get_comp_mut(
        &self,
        _ctype: TypeId
    ) -> Option<&RefCell<dyn NodeComponent>> {None}
    
    /// Called by [`NodeContext`] to fetch a async component for a descendant node (or the backbone).
    fn get_comp_arc(
        &self,
        _ctype: TypeId
    ) -> Option<Arc<dyn NodeComponentSync>> {None}
}

use downcast_rs::impl_downcast;
impl_downcast!(NodeHandler);

/// TODO: Implement.
pub trait NodeHandlerSync: NodeHandler + Send + Sync {
    
}

/// A potential pending request for the creation of a node-handler.
pub type NodeHandlerRequestRes = Result<NodeHandlerRequest, Box<dyn std::error::Error>>;

/// A pending request for the creation of a node-handler.
pub type NodeHandlerRequest = futures::channel::oneshot::Receiver<NodeHandlerCreated>;

/// The result of a completed [`NodeHandlerRequest`].
pub type NodeHandlerCreated = Result<NamedNodeHandlerBox, Box<dyn std::error::Error>>;

/// A box holding a [`NodeHandler`] instance.
pub type NodeHandlerBox = Box<dyn NodeHandler>;

/// A optional box holding an [`Event`].
pub type SubEvent = Option<Box<dyn Event>>;

/// A named [`NodeHandlerBox`].
#[derive(Debug)]
pub struct NamedNodeHandlerBox {
    /// The name(?) of the node.
    pub name: Arc<str>,
    /// The node.
    pub node: NodeHandlerBox
}

/// Node handler storage.
pub type Nodes = Vec<NamedNodeHandlerBox>;

/// A handler that does nothing.
pub mod empty {
    use super::*;
    
    /// An event-handler that does precisely nothing.
    #[derive(Debug)]
    pub struct EmptyEventHandler;
    impl NodeHandler for EmptyEventHandler {}
}

/// A handler that simply stores components.
pub mod cstore {
    use super::*;
    
    /// An event-handler that does precisely nothing.
    pub struct CStoreEventHandler {
        stored: std::collections::HashMap<std::any::TypeId, Box<dyn NodeComponent>>,
        celled: std::collections::HashMap<std::any::TypeId, Box<RefCell<dyn NodeComponent>>>,
        shared: std::collections::HashMap<std::any::TypeId, Arc<dyn NodeComponentSync>>,
    }
    
    impl CStoreEventHandler {
        /// Adds a new [`NodeComponent`] to this store.
        pub fn insert_box(&mut self, comp: Box<dyn NodeComponent>) -> bool {
            let type_id = comp.get_component_type_id();
            self.stored.insert(type_id, comp).is_none()
        }
        
        /// Adds a new [`NodeComponent`] to this store.
        pub fn insert_cell(&mut self, comp: Box<dyn NodeComponent>) -> bool {
            let type_id = comp.get_component_type_id();
            self.celled.insert(type_id, Box::new(RefCell::new(comp))).is_none()
        }
        
        /// Adds a new [`NodeComponent`] to this store.
        pub fn insert_arc(&mut self, comp: Arc<dyn NodeComponentSync>) -> bool {
            let type_id = comp.get_component_type_id();
            self.shared.insert(type_id, comp).is_none()
        }
    }
    
    impl CStoreEventHandler {
        /// Merges the given [`Self`] into this `self`.
        pub fn with(&mut self, other: CStoreEventHandler) {
            let Self {
                stored,
                celled,
                shared
            } = other;
            self.stored.extend(stored);
            self.celled.extend(celled);
            self.shared.extend(shared);
        }
        
    }
    
    impl Default for CStoreEventHandler {
        fn default() -> Self {
            Self {
                stored: Default::default(),
                celled: Default::default(),
                shared: Default::default(),
            }
        }
    }
    
    impl std::fmt::Debug for CStoreEventHandler {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("CStoreEventHandler")
                .field("box", &self.stored.keys())
                .field("cel", &self.celled.keys())
                .field("arc", &self.shared.keys())
                .finish()
        }
    }
    
    impl NodeHandler for CStoreEventHandler {
        fn get_comp(
            &self,
            ctype: TypeId
        ) -> Option<&dyn NodeComponent> {
            self.stored.get(&ctype).map(|comp| comp.as_ref())
        }

        fn get_comp_mut(
            &self,
            ctype: TypeId
        ) -> Option<&RefCell<dyn NodeComponent>> {
            self.celled.get(&ctype).map(|comp| comp.as_ref())
        }

        fn get_comp_arc(
            &self,
            ctype: TypeId
        ) -> Option<Arc<dyn NodeComponentSync>> {
            self.shared.get(&ctype).map(|comp| comp.clone())
        }
    }
}

/// A handler that combines two other handlers.
pub mod cascade {
    use super::*;
    
    /// An event-handler that does precisely nothing.
    #[derive(Debug)]
    pub struct CascadingEventHandler {
        /// The outer node.
        pub(crate) outer: NamedNodeHandlerBox,
        /// The inner node.
        pub(crate) inner: NamedNodeHandlerBox,
    }
    
    impl NodeHandler for CascadingEventHandler {
        fn handle_node_request<'e>(
            &'e mut self,
            name: Arc<str>,
            context: &'e mut NodeContext,
        ) -> NodeHandlerRequestRes {
            let inner_req = self.inner.node.handle_node_request(name.clone(), context);
            
            if let Err(_err) = inner_req {
                return self.outer.node.handle_node_request(name, context);
            }
            
            inner_req
        }
        
        fn handle_event<'e>(
            &'e mut self,
            event: &'e mut EventWrapper,
            context: &'e mut NodeContext,
        ) -> SubEvent {
            match event.get_phase() {
                EventPhase::Creation => unreachable!("This should never ever occur"),
                EventPhase::Falling => {
                    // TODO: Handle sub-event returned by the outer node.
                    self.outer.node.handle_event(event, context);
                    if !event.can_fall() {return None}
                    self.inner.node.handle_event(event, context)
                },
                EventPhase::Acting => {
                    self.inner.node.handle_event(event, context)
                },
                EventPhase::Rising => {
                    // TODO: Handle sub-event returned by the inner node.
                    self.inner.node.handle_event(event, context);
                    if !event.can_rise() {return None}
                    self.outer.node.handle_event(event, context)
                },
            }
        }
        
        fn get_comp(
            &self,
            ctype: TypeId
        ) -> Option<&dyn NodeComponent> {
            self.inner.node.get_comp(ctype).or_else(||self.outer.node.get_comp(ctype))
        }
        
        fn get_comp_mut(
            &self,
            ctype: TypeId
        ) -> Option<&RefCell<dyn NodeComponent>> {
            self.inner.node.get_comp_mut(ctype).or_else(||self.outer.node.get_comp_mut(ctype))
        }
        
        fn get_comp_arc(
            &self,
            ctype: TypeId
        ) -> Option<Arc<dyn NodeComponentSync>> {
            self.inner.node.get_comp_arc(ctype).or_else(||self.outer.node.get_comp_arc(ctype))
        }
    }

}