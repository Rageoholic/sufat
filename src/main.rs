/*
This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::Arc;

use log::debug;
use render_context::RenderContext;
use winit::{
    dpi::{LogicalSize, Size},
    event::{Event, StartCause, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
mod render_context;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = Arc::new(
        WindowBuilder::new()
            .with_visible(false)
            .with_inner_size(Size::Logical(LogicalSize::new(1280f64, 720f64)))
            .build(&event_loop)
            .unwrap(),
    );

    let render_context = RenderContext::new(window.clone()).unwrap();

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
            //temporary capture so we can make sure that render_context is moved
            //in
            Some(&render_context);
            debug!("Loop exiting");
        }
        _ => {}
    })
}
