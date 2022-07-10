//! Node Context
use crate::*;

/// Node Context: A disjoint subset of the ['Backbone'].
/// 
/// Used by [`NodeHandler`]'s to fetch [`NodeComponent`]'s from the backbone to interact with.
pub struct NodeContext<'c> {
    /// Name of the current node.
    pub name: Arc<str>,
    
    /// The nodes that are not current / above the current node.
    pub cons: &'c mut [NamedNodeHandlerBox],
}

impl<'c> NodeContext<'c> {
    
    /// Returns the fully formed name for a child with the given partial name.
    pub fn get_child_name(&self, name: &str) -> Arc<str> {
        let pname = if self.name.as_ref() == "/" {""} else {self.name.as_ref()};
        Arc::from(format!("{pname}/{name}"))
    }
    
    /// Returns a reference to a [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component<C: NodeComponent + 'static>(&self) -> Option<&C> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter().rev() {
            if let Some(c) = node.node.get_comp(type_id) {
                return c.downcast_ref()
            }
        }
        
        None
    }
    
    /// Returns a [`std::cell::RefCell`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component_mut<C: NodeComponent + 'static>(&mut self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter_mut().rev() {
            if let Some(c) = node.node.get_comp_mut(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns an [`std::sync::Arc`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component_arc<C: NodeComponentSync + 'static>(&mut self) -> Option<Arc<C>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter_mut().rev() {
            if let Some(c) = node.node.get_comp_arc(type_id) {
                match c.into_any_arc().downcast::<C>() {
                    Ok(c) => return Some(c),
                    Err(_e) => continue,
                }
            }
        }
        
        None
    }
    
}

/// Outer Node Context: A [`NodeContext`] paired with a 'current' node.
pub struct OuterNodeContext<'c> {
    /// The partial/disjoint backbone.
    pub(crate) context: NodeContext<'c>,
    
    /// The currently active node.
    pub(crate) current: &'c mut NamedNodeHandlerBox
}

// TODO: Does it make sense to deref the NodeContext?
impl<'c> std::ops::Deref for OuterNodeContext<'c> {
    type Target = NodeContext<'c>;
    
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<'c> std::ops::DerefMut for OuterNodeContext<'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<'c> OuterNodeContext<'c> {
    
    /// Returns the fully formed name for a child with the given partial name.
    pub fn get_child_name(&self, name: &str) -> Arc<str> {
        self.context.get_child_name(name)
    }
    
    /// Returns a new [`OuterNodeContext`] that is a subset of this context *excluding* the current node.
    pub fn get_subcontext_at(&mut self, at: usize) -> Option<OuterNodeContext> {
        if at > self.context.cons.len() {return None}
        let (start, end) = self.context.cons.split_at_mut(at);
        
        Some(OuterNodeContext {
            context: NodeContext {
                name: end[0].name.clone(),
                cons: &mut * start,
            },
            current: &mut end[0]
        })
    }
    
    /// Fires an [`Event`] to run trough the backbone to the current node.
    pub fn process_event<E: Event>(&mut self, event: &mut E) {
        self.process_event_wrapper(EventWrapper::new(event));
    }
    
    /// Fires an [`Event`], wrapped in a [`EventWrapper`], to run trough the backbone to the current node.
    pub fn process_event_wrapper(&mut self, mut wrapper: EventWrapper) {
        
        // This walker will FALL down the backbone...
        let falling_walker = (0..self.context.cons.len())
            .map(|idx| (Some(idx), EventPhase::Falling))
        ;
        
        // This walker will ACT on the bottom of the backbone...
        let acting_walker = std::iter::once
            ((None, EventPhase::Acting))
        ;
        
        // This walker will RISE up thru the backbone...
        let rising_walker = (0..self.context.cons.len())
            .rev()
            .map(|idx| (Some(idx), EventPhase::Rising))
        ;
        
        // This iterator will first FALL, then ACT
        // and finally RISE, in that exact order.
        let walker = falling_walker
            .chain(acting_walker)
            .chain(rising_walker)
        ;
        
        // Ready, set, iterate!
        for (idx, phase) in walker {
            match &phase {
                // Should never happen; but handle it anyway! ¯\_(ツ)_/¯
                EventPhase::Creation => continue,
                
                // Check if any previous iteration of the FALL-phase cancelled falling
                EventPhase::Falling if !wrapper.can_fall() => continue,
                
                // Check if any previous iteration cancelled acting
                EventPhase::Acting if !wrapper.can_eval() => continue,
                
                // Check if any previous iteration cancelled rising
                EventPhase::Rising if !wrapper.can_rise() => continue,
                
                // proceed with event handling
                _ => ()
            }
            
            // Phase change? Update wrapper!
            if phase > wrapper.get_phase() {
                wrapper = wrapper.into_phase(phase);
            }
            
            if let Some(idx) = idx {
                if let Some(mut subctx) = self.get_subcontext_at(idx) {
                    subctx.current.node.handle_event(&mut wrapper, &mut subctx.context);
                }
            } else {
                // This branch only get's called once,
                // during the ACT phase, precisely between FALL and RISE.
                self.current.node.handle_event(&mut wrapper, &mut self.context);
            }
        }
        
        // We are done!
    }
}

impl Backbone {
    /// Returns an [`OuterNodeContext`] focused on the root node, if there is one.
    pub fn get_root_context(&mut self) -> Option<OuterNodeContext> {
        
        let first = match self.nodes.first_mut() {
            Some(s) => s,
            None => return None,
        };
        
        Some(OuterNodeContext {
            context: NodeContext {
                name: first.name.clone(),
                cons: &mut[],
            },
            current: first
        })
    }
    
    /// Returns an [`OuterNodeContext`] focused on the current node, if there is one.
    pub fn get_context(&mut self) -> Option<OuterNodeContext> {
        
        let (last, cons) = match self.nodes.split_last_mut() {
            Some(s) => s,
            None => return None,
        };
        
        Some(OuterNodeContext {
            context: NodeContext {
                name: last.name.clone(),
                cons: &mut * cons,
            },
            current: last
        })
    }
    
    /// Returns an [`OuterNodeContext`] focused on the node indicated by the `at`-parameter.
    pub fn get_context_at(&mut self, at: usize) -> Option<OuterNodeContext> {
        if at > self.nodes.len() {return None}
        let (start, end) = self.nodes.split_at_mut(at);
        
        Some(OuterNodeContext {
            context: NodeContext {
                name: end[0].name.clone(),
                cons: &mut * start,
            },
            current: &mut end[0]
        })
    }
    
    // TODO: Implement a `get_context_for(PATH)`-method.
    
    /// Returns a reference to a [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component<C: NodeComponent + 'static>(&self) -> Option<&dyn NodeComponent> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter().rev() {
            if let Some(c) = node.node.get_comp(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns a [`std::cell::RefCell`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component_mut<C: NodeComponent + 'static>(&self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter().rev() {
            if let Some(c) = node.node.get_comp_mut(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns an [`std::sync::Arc`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component_arc<C: NodeComponentSync + 'static>(&mut self) -> Option<Arc<dyn NodeComponentSync>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.nodes.iter_mut().rev() {
            if let Some(c) = node.node.get_comp_arc(type_id) {
                return Some(c)
            }
        }
        
        None
    }
}
