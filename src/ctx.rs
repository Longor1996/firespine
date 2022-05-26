//! Node Context
use crate::*;

/// Node Context: A disjoint set of the backbone struct.
/// 
/// Used by events to fetch components from the backbone to interact with.
pub struct NodeContext<'c> {
    /// Name of the current node.
    pub name: Arc<str>,
    
    /// The nodes that are not current.
    pub cons: &'c mut [NodeHandlerBox],
}

impl<'c> NodeContext<'c> {
    /// Returns a component of the given type, if possible.
    pub fn get_component<C: NodeComponent + 'static>(&self) -> Option<&dyn NodeComponent> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter().rev() {
            if let Some(c) = node.1.get_comp(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns a component of the given type, if possible.
    pub fn get_component_mut<C: NodeComponent + 'static>(&mut self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter_mut().rev() {
            if let Some(c) = node.1.get_comp_mut(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns a component of the given type, if possible.
    pub fn get_component_arc<C: NodeComponentSync + 'static>(&mut self) -> Option<Arc<dyn NodeComponentSync>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter_mut().rev() {
            if let Some(c) = node.1.get_comp_arc(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
}

/// Outer Node Context: A disjoint set of the backbone struct and a 'current' node.
pub struct OuterNodeContext<'c> {
    pub(crate) inner: NodeContext<'c>,
    pub(crate) outer: &'c mut NodeHandlerBox
}

impl<'c> std::ops::Deref for OuterNodeContext<'c> {
    type Target = NodeContext<'c>;
    
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'c> std::ops::DerefMut for OuterNodeContext<'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'c> OuterNodeContext<'c> {
    
    /// Returns the fully formed name for a child with the given partial name.
    pub fn get_child_name(&self, name: &str) -> Arc<str> {
        let pname = if self.inner.name.as_ref() == "/" {""} else {self.inner.name.as_ref()};
        Arc::from(format!("{pname}/{name}"))
    }
    
    /// Returns a new context that is a subset of this context *without* the current node.
    pub fn get_subset(&mut self, at: usize) -> Option<OuterNodeContext> {
        if at > self.inner.cons.len() {return None}
        let (start, end) = self.inner.cons.split_at_mut(at);
        
        Some(OuterNodeContext {
            inner: NodeContext {
                name: end[0].0.clone(),
                cons: &mut * start,
            },
            outer: &mut end[0]
        })
    }
    
    /// Fires an event to run trough the backbone to the current node.
    pub fn process_event<E: Event>(&mut self, event: &mut E) {
        self.process_event_wrapper(EventWrapper::new(event));
    }
    
    /// Fires a wrapped event to run trough the backbone to the current node.
    pub fn process_event_wrapper(&mut self, mut wrapper: EventWrapper) {
        wrapper = wrapper.into_phase(EventPhase::Falling);
        
        for idx in 0..self.inner.cons.len() {
            if !wrapper.can_fall() {
                break;
            }
            
            if let Some(mut subctx) = self.get_subset(idx) {
                subctx.outer.1.handle_event(&mut wrapper, &mut subctx.inner);
            }
        }
        
        if wrapper.can_eval() {
            wrapper = wrapper.into_phase(EventPhase::Acting);
            self.outer.1.handle_event(&mut wrapper, &mut self.inner);
        }
        
        wrapper = wrapper.into_phase(EventPhase::Rising);
        for idx in (0..self.inner.cons.len()).rev() {
            if !wrapper.can_rise() {
                break;
            }
            
            if let Some(mut subctx) = self.get_subset(idx) {
                subctx.outer.1.handle_event(&mut wrapper, &mut subctx.inner);
            }
        }
        
        // We are done!
    }
}

impl Backbone {
    /// Returns a node-context focused on the root node, if there is one.
    pub fn get_root_context(&mut self) -> Option<OuterNodeContext> {
        
        let first = match self.nodes.first_mut() {
            Some(s) => s,
            None => return None,
        };
        
        Some(OuterNodeContext {
            inner: NodeContext {
                name: first.0.clone(),
                cons: &mut[],
            },
            outer: first
        })
    }
    
    /// Returns a node-context focused on the current node, if there is one.
    pub fn get_context(&mut self) -> Option<OuterNodeContext> {
        
        let (last, cons) = match self.nodes.split_last_mut() {
            Some(s) => s,
            None => return None,
        };
        
        Some(OuterNodeContext {
            inner: NodeContext {
                name: last.0.clone(),
                cons: &mut * cons,
            },
            outer: last
        })
    }
    
    /// Returns a node-context focused on the node indicated by the `at`-parameter.
    pub fn get_subset(&mut self, at: usize) -> Option<OuterNodeContext> {
        if at > self.nodes.len() {return None}
        let (start, end) = self.nodes.split_at_mut(at);
        
        Some(OuterNodeContext {
            inner: NodeContext {
                name: end[0].0.clone(),
                cons: &mut * start,
            },
            outer: &mut end[0]
        })
    }
    
    
    
    /// Returns a component of the given type, if possible.
    pub fn get_component<C: NodeComponent + 'static>(&self) -> Option<&dyn NodeComponent> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter().rev() {
            if let Some(c) = node.1.get_comp(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns a component of the given type, if possible.
    pub fn get_component_mut<C: NodeComponent + 'static>(&self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter().rev() {
            if let Some(c) = node.1.get_comp_mut(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns a component of the given type, if possible.
    pub fn get_component_arc<C: NodeComponentSync + 'static>(&mut self) -> Option<Arc<dyn NodeComponentSync>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter_mut().rev() {
            if let Some(c) = node.1.get_comp_arc(type_id) {
                return Some(c)
            }
        }
        
        None
    }
}