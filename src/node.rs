//! Nodes

use crate::*;

/// All user-logic and -state for a node is owned by it's associated handler.
pub trait NodeHandler: std::fmt::Debug + Send + Sync {
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
    ) -> NodeHandlerRequest {
        Err("Node has no children".to_owned())
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
    ) {
        if !event.is_silent() {
            crate::debug!(
                "EVENT {} AT {}: {:?}",
                event.get_phase(),
                context.name,
                event.get_event()
            );
        }
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

/// A potential future for the creation of a node-handler.
pub type NodeHandlerRequest = Result<NodeHandlerAwaiter, String>;

/// A future for the creation of a node-handler.
pub type NodeHandlerAwaiter = futures::channel::oneshot::Receiver<NodeHandlerCreated>;

/// Result of node handler creation.
pub type NodeHandlerCreated = Result<NodeHandlerBox, String>;

/// A box holding a NodeHandler instance.
pub type NodeHandlerBox = (Arc<str>, Box<dyn NodeHandler>);

/// Node handler storage.
pub type Nodes = Vec<NodeHandlerBox>;

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
        outer: NodeHandlerBox,
        inner: NodeHandlerBox,
    }
    
    impl NodeHandler for CascadingEventHandler {
        fn handle_node_request<'e>(
            &'e mut self,
            name: Arc<str>,
            context: &'e mut NodeContext,
        ) -> NodeHandlerRequest {
            let inner_req = self.inner.1.handle_node_request(name.clone(), context);
            
            if let Err(_err) = inner_req {
                return self.outer.1.handle_node_request(name, context);
            }
            
            inner_req
        }

        fn handle_event<'e>(
            &'e mut self,
            event: &'e mut EventWrapper,
            context: &'e mut NodeContext,
        ) {
            match event.get_phase() {
                EventPhase::Creation => unreachable!("This should never ever occur"),
                EventPhase::Falling => {
                    self.outer.1.handle_event(event, context);
                    if !event.can_fall() {return}
                    self.inner.1.handle_event(event, context);
                },
                EventPhase::Acting => {
                    self.inner.1.handle_event(event, context);
                },
                EventPhase::Rising => {
                    self.inner.1.handle_event(event, context);
                    if !event.can_rise() {return}
                    self.outer.1.handle_event(event, context);
                },
            }
        }
    }

}