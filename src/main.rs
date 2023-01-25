#![allow(unused)]

mod events;
mod generic_result;
mod listener;
mod lua;
mod module;

use async_std::{channel::unbounded, task};
use events::{ListenerEvent, LuaEvent};
use futures::Future;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn spawn<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{e}");
        }
    })
}

pub async fn block_on<F>(fut: F)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::block_on(async move {
        if let Err(e) = fut.await {
            eprintln!("{e}");
        }
    });
}

fn main() -> generic_result::GenericResult<()> {
    let (listener_sender, listener_receiver) = unbounded::<ListenerEvent>();
    let (lua_sender, lua_receiver) = unbounded::<LuaEvent>();

    task::spawn(lua::run(lua_sender, listener_receiver));
    task::block_on(listener::run(listener_sender, lua_receiver));

    // task::block_on(async move { futures::join!(lua_handle, listener_handle) });

    Ok(())
}
