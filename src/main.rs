mod listener;
mod lua;
mod module;

use async_std::{channel::unbounded, task};

fn main() {
    let (listener_sender, listener_receiver) = unbounded::<listener::Command>();
    let (lua_sender, lua_receiver) = unbounded::<lua::Command>();

    task::spawn(lua::run(listener_sender.clone(), lua_receiver));
    task::block_on(listener::run(
        lua_sender,
        listener_sender,
        listener_receiver,
    ));
}
