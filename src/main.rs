use log::debug;
use render_context::RenderContext;
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
mod render_context;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();

    let _render_context = RenderContext::new(&window).unwrap();

    event_loop.run(move |event, _target, control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            control_flow.set_poll();
            window.set_visible(true)
        }
        Event::WindowEvent { window_id, event } => {
            if event == WindowEvent::CloseRequested && window_id == window.id()
            {
                debug!("recieved shutdown request");
                control_flow.set_exit();
                window.set_visible(false);
            }
        }
        Event::LoopDestroyed => {
            debug!("Loop exiting");
        }
        _ => {}
    })
}
