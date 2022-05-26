//! Backbone navigation by means of thunks.
use crate::*;
use std::{collections::VecDeque};

/// Queue of thunks.
pub type Thunks = VecDeque<Thunk>;

/// A thunk is a navigational action within the backbone.
pub enum Thunk {
    /// Navigate to root.
    ToRoot,
    
    /// Navigate to parent.
    ToParent,
    
    /// Navigate to self (no-op).
    ToSelf,
    
    /// Navigate to sub-node.
    ToNode(String),
    
    /// Waiting for a node to construct itself.
    Waiting(Arc<str>, NodeHandlerRequest),
    
    /// Something went horribly wrong.
    Error(Box<dyn std::error::Error>),
    
    /// Navigation completion.
    End
}

impl Thunk {
    /// Tries to parse a Thunk out of the given path.
    ///
    /// Returns both a thunk and the remaining path.
    pub fn parse(path: &str) -> Option<(Thunk, &str)> {
        let path = path.trim();
        
        if path.is_empty() {
            return None
        }
        
        if let Some(path) = path.strip_prefix('/') {
            return Some((Thunk::ToRoot, path.trim_start_matches('/')))
        }
        
        if let Some(path) = path.strip_prefix("..") {
            return Some((Thunk::ToParent, path.trim_start_matches('/')))
        }
        
        if let Some(path) = path.strip_prefix('.') {
            return Some((Thunk::ToSelf, path.trim_start_matches('/')))
        }
        
        if let Some((current, next)) = path.split_once('/') {
            return Some((Thunk::ToNode(current.to_owned()), next))
        }
        
        Some((Thunk::ToNode(path.to_owned()), ""))
    }
}

impl std::fmt::Display for Thunk {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::ToRoot => write!(fmt, "/"),
			Self::ToSelf => write!(fmt, "./"),
			Self::ToParent => write!(fmt, "../"),
			Self::ToNode(name) => write!(fmt, "{name}"),
			Self::Waiting(_, _) => write!(fmt, "!Waiting"),
			Self::Error(e) => write!(fmt, "!Error: {e}"),
			Self::End => write!(fmt, "!Ok"),
		}
	}
}

/// Node-related events.
mod events {
    /// Event that is fired when navigation finishes.
    #[derive(Debug)]
    pub struct NavigationCompletionEvent;
    impl crate::Event for NavigationCompletionEvent {}
}


// Thunk handling.
impl Backbone {
    /// Navigate to a different path.
    pub fn navigate(&mut self, mut path: &str) {
        let original_path = path;
        let current_path = self.path_as_string();
        
        // Inexact test to avoid moving to the current node...
        if current_path == path {
            crate::warn!("Attempted to navigate to current path; ignoring command.");
            return;
        }
        
        // Avoid infinite movement.
        if self.thunks.len() > 16 {
            return;
        }
        
        // TODO: Implement CANCELLABLE NavigationBeginningEvent right about here.
        
        while let Some((thunk, subpath)) = Thunk::parse(path) {
            path = subpath;
            self.thunks.push_back(thunk);
        }
        
        self.thunks.push_back(Thunk::End);
        crate::info!("Navigating from '{current_path}' to '{original_path}': {} thunks in queue: {}", self.thunks.len(), self.thunks_as_string());
    }
    
    /// Returns if the backbone is navigating.
    pub fn is_moving(&self) -> bool {
        ! self.thunks.is_empty()
    }
    
    /// Returns if the backbone is idle.
    pub fn is_idle(&self) -> bool {
        self.thunks.is_empty()
    }
    
    /// Returns the current path as a [`String`].
    pub fn path_as_string(&self) -> String {
        let mut out = String::new();
        let mut first = true;
        for node in &self.nodes {
            if ! first {
                out.push('/');
            }
            out.push_str(&node.0);
            first = false;
        }
        out
    }
    
    /// Returns the current thunk queue as a [`String`]; for debugging.
    pub fn thunks_as_string(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for thunk in self.thunks.iter() {
            write!(out, "{thunk} ").unwrap();
        }
        
        out
    }
    
    pub(crate) fn process_thunks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let pthunk: Option<Thunk> = match self.thunks.pop_front() {
            None => return Ok(()),
            
            Some(Thunk::End) => {
                crate::info!("Navigation Complete: {}", self.path_as_string());
                self.get_context().unwrap().process_event(&mut events::NavigationCompletionEvent);
                None
            },
            
            Some(Thunk::Error(error)) => {
                return Err(error);
            },
            
            Some(Thunk::Waiting(nid, mut rx)) => {
                match rx.try_recv() {
                    Ok(response) => {
                        match response {
                            Some(result) => match result {
                                Ok(node) => {
                                    // Insert and jump into node...
                                    self.nodes.push(node);
                                    // TODO: Fire event on node insertion.
                                    None
                                },
                                Err(err) => {
                                    Some(Thunk::Error(err.into()))
                                },
                            },
                            None => {
                                Some(Thunk::Waiting(nid, rx))
                            },
                        }
                    },
                    Err(err) => {
                        Some(Thunk::Error(err.into()))
                    },
                }
            },
            
            Some(Thunk::ToNode(nn)) => {
                // Construct full name of child-node...
                let mut ctx = self.get_context().unwrap();
                let child_name = ctx.get_child_name(&nn);
                
                match ctx.current.1.handle_node_request(child_name.clone(), &mut ctx.context) {
                    Err(err) => {
                        Some(Thunk::Error(err.into()))
                    },
                    Ok(rx) => {
                        Some(Thunk::Waiting(child_name, rx))
                    },
                }
            },
            
            Some(Thunk::ToSelf) => {
                // This is a no-op.
                None
            },
            
            Some(Thunk::ToParent) => {
                // Don't pop the root!
                if self.nodes.len() > 1 {
                    self.nodes.pop();
                    // TODO: Fire event on node exit.
                    // TODO: Destroy the node.
                }
                None
            },
            
            Some(Thunk::ToRoot) => {
                if self.nodes.len() > 1 {
                    // Not yet at root, keep popping...
                    Some(Thunk::ToParent)
                    // FIXME: This is a bit... silly. Improve later.
                } else {
                    // reached root!
                    None
                }
            },
        };
        
        if let Some(pthunk) = pthunk {
            self.thunks.push_front(pthunk)
        }
        
        // All is okay.
        Ok(())
    }
}
