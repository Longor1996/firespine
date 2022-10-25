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

impl<'c> std::fmt::Debug for NodeContext<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeContext")
            .field("name", &self.name)
            .field("prev", &self.cons).finish()
    }
}

impl<'c> NodeContext<'c> {
    
    /// Returns the fully formed name for a child with the given partial name.
    pub fn get_child_name(&self, name: &str) -> Arc<str> {
        let pname = if self.name.as_ref() == "/" {""} else {self.name.as_ref()};
        Arc::from(format!("{pname}/{name}"))
    }
    
    /// Returns a reference to a [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_cons_component<C: NodeComponent + 'static>(&self) -> Option<&C> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter().rev() {
            if let Some(c) = node.node.get_comp(type_id) {
                return c.downcast_ref()
            }
        }
        
        None
    }
    
    /// Returns a [`std::cell::RefCell`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_cons_component_mut<C: NodeComponent + 'static>(&self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter().rev() {
            if let Some(c) = node.node.get_comp_mut(type_id) {
                return Some(c)
            }
        }
        
        None
    }
    
    /// Returns an [`std::sync::Arc`]'d [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_cons_component_arc<C: NodeComponentSync + 'static>(&self) -> Option<Arc<C>> {
        let type_id = TypeId::of::<C>();
        
        for node in self.cons.iter().rev() {
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

impl<'c> std::fmt::Debug for OuterNodeContext<'c> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OuterNodeContext")
            .field("context", &self.context)
            .field("current", &self.current).finish()
    }
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
    
    /// Returns a mutable reference to the current node.
    pub fn get_current_node(&mut self) -> &mut NamedNodeHandlerBox {
        self.current
    }
    
    /// Returns the fully formed name for a child with the given partial name.
    pub fn get_child_name(&self, name: &str) -> Arc<str> {
        self.context.get_child_name(name)
    }
    
    /// Returns a new [`OuterNodeContext`] that is a subset of this context, BEFORE `at`.
    /// 
    /// The current node of this context is *excluded* from the result.
    pub fn get_subcontext_before(&mut self, at: usize) -> Option<OuterNodeContext> {
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
    
    /// Returns a new [`OuterNodeContext`] that is a subset of this context, AFTER `at`.
    /// 
    /// The current node of this context is preserved.
    pub fn get_subcontext_after(&mut self, at: usize) -> Option<OuterNodeContext> {
        if at > self.context.cons.len() {return None}
        let (_start, end) = self.context.cons.split_at_mut(at);
        
        Some(OuterNodeContext {
            context: NodeContext {
                name: end[0].name.clone(),
                cons: &mut * end,
            },
            current: self.current
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
        let mut walker = std::iter::empty()
            .chain(falling_walker)
            .chain(acting_walker)
            .chain(rising_walker)
            .peekable()
        ;
        
        // Ready, set, iterate!
        loop {
            let (idx, phase) = match walker.next() {
                Some(s) => s,
                None => break,
            };
            
            if !wrapper.can(phase) {
                continue;
            }
            
            wrapper.next_phase(phase); //if?
            
            let sub = if let Some(idx) = idx {
                if let Some(mut subctx) = self.get_subcontext_before(idx) {
                    subctx.current.node.handle_event(&mut wrapper, &mut subctx.context)
                } else {
                    None
                }
            } else {
                // This branch only get's called once,
                // during the ACT phase, precisely between FALL and RISE.
                self.current.node.handle_event(&mut wrapper, &mut self.context)
            };
            
            if let Some(mut sub_event) = sub {
                // This makes events returned by the ACT-phase a no-op
                if let Some(idx) = idx {
                    if let Some(mut ctx) = self.get_subcontext_after(idx + 1) {
                        // TODO: This impl is terrible. Find a better way.
                        ctx.process_event_wrapper(EventWrapper {
                            event: sub_event.as_mut(),
                            ..wrapper
                        });
                    }
                }
            }
        }
        
        // We are done!
    }
    
    /// Returns a reference to a [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component<C: NodeComponent + 'static>(&self) -> Option<&C> {
        let type_id = TypeId::of::<C>();
        self.current.node.get_comp(type_id)
            .and_then(|c| c.downcast_ref::<C>())
            .or_else(||self.context.get_cons_component::<C>())
    }
    
    /// Returns a reference to a [`NodeComponent`] of the given type `C`, if one exists.
    pub fn get_component_mut<C: NodeComponent + 'static>(&self) -> Option<&RefCell<dyn NodeComponent>> {
        let type_id = TypeId::of::<C>();
        self.current.node.get_comp_mut(type_id)
            .or_else(||self.context.get_cons_component_mut::<C>())
    }
    
    /// Returns a reference to a [`NodeComponentSync`] of the given type `C`, if one exists.
    pub fn get_component_arc<C: NodeComponentSync + 'static>(&self) -> Option<Arc<C>> {
        let type_id = TypeId::of::<C>();
        self.current.node.get_comp_arc(type_id)
            .and_then(|c| c.into_any_arc().downcast::<C>().ok())
            .or_else(||self.context.get_cons_component_arc::<C>())
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
    pub(crate) fn get_subcontext_before(&mut self, at: usize) -> Option<OuterNodeContext> {
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
