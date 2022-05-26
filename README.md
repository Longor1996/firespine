# Firespine

A relatively simple framework-as-a-library for application-state, -routing and event-handling.

## Usage

```rust
use firespine::{Backbone, NodeHandler};

#[derive(Debug)]
struct MyNodeHandler;
impl NodeHandler for MyNodeHandler {
    // Override handler methods as needed!
    
    // For example, to receive events:
    fn handle_event<'e>(
        &'e mut self,
        event: &'e mut EventWrapper,
        context: &'e mut NodeContext,
    ) {
        if let Some(my_event) = event.downcast_ref::<MyEvent>() {
            // Handle the event! :D
        }
    }
}

#[derive(Debug)]
struct MyEvent;
impl Event for MyEvent {}

fn main() {
    let mut backbone = Backbone::from(MyNodeHandler);
    
    // 'navigate' to a path in the backbone.
    backbone.navigate("/local/world-3/play");
    
    loop {
        // Regularly call `update` in your mainloop...
        backbone.update().expect("backbone error");
        
        /// ...and fire away as you wish!
        backbone.get_context().process_event(&mut MyEvent);
    }
}
```
