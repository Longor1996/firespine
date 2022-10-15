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

/// A handler that combines two other handlers.
pub mod cascade {
    use super::*;
    
    /// An event-handler that does precisely nothing.
    #[derive(Debug)]
    pub struct CascadingEventHandler {
        outer: NamedNodeHandlerBox,
        inner: NamedNodeHandlerBox,
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
    }

}