//! Events
use crate::*;

/// An event.
pub trait Event: Downcast + std::fmt::Debug {
    /// Should the event not be logged?
    fn is_silent(&self) -> bool {false}
}

use downcast_rs::impl_downcast;
impl_downcast!(Event);

/// A basic empty event.
#[derive(Debug)]
pub struct EmptyEvent;
impl Event for EmptyEvent {}

/// Wraps an event as it is processed by the backbone.
pub struct EventWrapper<'e> {
    /// The event being processed.
    event: &'e mut dyn Event,

    // State of the event.
    phase: EventPhase,

    /// Can the event flow towards its destination?
    can_fall: bool,

    /// Can the event be evaluated by its destination?
    can_eval: bool,

    /// Can the event flow back towards its source?
    can_rise: bool,
}

impl<'e> EventWrapper<'e> {
    /// Wraps the given [`Event`] in a fresh [`EventWrapper`].
    pub fn new(event: &'e mut dyn Event) -> Self {
        Self {
            event,
            phase: EventPhase::Creation,
            can_fall: true,
            can_eval: true,
            can_rise: true,
        }
    }
    
    /// Prevents the event from being evaluated by its destination.
    pub fn prevent_action(&mut self) {
        self.can_eval = false;
    }
    
    /// Are we allowed to eval?
    pub fn can_eval(&self) -> bool {
        self.can_eval
    }
    
    /// Stops the flow of the event toward its destination.
    pub fn stop_falling(&mut self) {
        self.can_fall = false;
    }
    
    /// Are we allowed to keep falling?
    pub fn can_fall(&self) -> bool {
        self.can_fall
    }
    
    /// Stops the flow of the event back towards its source.
    pub fn stop_rising(&mut self) {
        self.can_rise = false;
    }
    
    /// Are we allowed to keep rising?
    pub fn can_rise(&self) -> bool {
        self.can_rise
    }
    
    /// Stop the event completely.
    pub fn stop(&mut self) {
        self.can_fall = false;
        self.can_eval = false;
        self.can_rise = false;
    }
    
    /// Returns the `EventPhase` the event is currently in.
    pub fn get_phase(&self) -> EventPhase {
        self.phase
    }
    
    /// Returns the wrapped `Event`.
    pub fn get_event(&self) -> &dyn Event {
        self.event
    }
    
    /// Move into another phase.
    pub fn into_phase(self, phase: EventPhase) -> Self {
        Self {phase, ..self}
    }
}

impl<'e> From<&'e mut dyn Event> for EventWrapper<'e> {
    fn from(event: &'e mut dyn Event) -> Self {
        Self::new(event)
    }
}

impl<'e> std::ops::Deref for EventWrapper<'e> {
    type Target = dyn Event;

    fn deref(&self) -> &Self::Target {
        self.event
    }
}

impl<'e> std::ops::DerefMut for EventWrapper<'e> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.event
    }
}

/// Represents the phase (or state) of an event as it's being processed.
#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug, Ord, PartialOrd)]
pub enum EventPhase {
    /// The event is being wrapped in a `EventWrapper`.
    Creation = 1,

    /// The event is flowing towards its destination.
    Falling = 2,

    /// The event is being evaluated by its destination.
    Acting = 3,

    /// The event is flowing back towards its source.
    Rising = 4,
}

impl std::fmt::Display for EventPhase {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Creation => write!(fmt, "Creation"),
            Self::Falling => write!(fmt, "Falling"),
            Self::Acting => write!(fmt, "Acting"),
            Self::Rising => write!(fmt, "Rising"),
        }
    }
}
