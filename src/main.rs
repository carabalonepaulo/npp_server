#![allow(unused)]

mod events;
mod generic_result;
mod listener;
mod lua;

use async_std::{channel::unbounded, task};
use events::{ListenerEvent, LuaEvent};

fn main() -> generic_result::GenericResult<()> {
    let (listener_sender, listener_receiver) = unbounded::<ListenerEvent>();
    let (lua_sender, lua_receiver) = unbounded::<LuaEvent>();

    task::spawn(lua::run(lua_sender, listener_receiver));
    task::block_on(listener::run(listener_sender, lua_receiver));

    Ok(())
}
